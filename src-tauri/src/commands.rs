//! The `#[tauri::command]` surface of the IPC API (TECH.md §5). Repository
//! work runs on the blocking thread pool.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use specta_typescript::Typescript;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_specta::{Builder, collect_commands, collect_events};
use turbodiff_repo::{CommitItem, Comparison, DiffOptions, FileDiff, RefItem, Side};

use crate::app::placement::Target;
use crate::app::recent::RecentRepo;
use crate::app::settings::{Settings, SettingsPatch};
use crate::app::{self, AppState, windows};
use crate::error::{AppError, AppResult};
use crate::events::{OpenFailed, RecentChanged, RepoOpened, SettingsChanged};
use crate::session::{ChangeSet, Session, Versioned};

/// Where the generated TypeScript bindings live.
pub const TYPES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/types.ts");

/// What a window shows when its page loads.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    /// `None` for the welcome screen.
    pub repo: Option<RepoOpened>,
    /// Why opening a folder failed, for a window opened to show that error.
    /// Returned once.
    pub error: Option<AppError>,
}

/// Returns the app version from `Cargo.toml`.
#[tauri::command]
#[specta::specta]
pub async fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// The repository this window shows, if any.
#[tauri::command]
#[specta::specta]
pub async fn get_window_state(app: AppHandle, window: WebviewWindow) -> AppResult<WindowState> {
    let (session, error) = {
        let state = app.state::<AppState>();
        let mut windows = state.windows();
        match windows.get_mut(window.label()) {
            Some(slot) => (slot.session.clone(), slot.pending_error.take()),
            None => (None, None),
        }
    };
    let repo = match session {
        Some(session) => Some(
            blocking(move || {
                Ok(RepoOpened {
                    repo: session.repo().info()?,
                    comparison: session.comparison(),
                })
            })
            .await?,
        ),
        None => None,
    };
    Ok(WindowState { repo, error })
}

/// Opens the repository containing `path`: in this window if it shows the
/// welcome screen, otherwise in a new window. A repository that's already
/// open focuses its window. The window that shows it gets `repo-opened`.
#[tauri::command]
#[specta::specta]
pub async fn open_repo(app: AppHandle, window: WebviewWindow, path: PathBuf) -> AppResult<()> {
    let target = Target::Window(window.label().to_owned());
    windows::open(&app, path, None, target).await
}

/// Shows the native folder picker and opens the chosen folder as `open_repo`
/// does. Errors arrive as `open-failed`.
#[tauri::command]
#[specta::specta]
pub async fn show_open_dialog(app: AppHandle, window: WebviewWindow) {
    windows::choose_folder(&app, Some(window.label().to_owned()));
}

#[tauri::command]
#[specta::specta]
pub async fn list_branches(
    app: AppHandle,
    window: WebviewWindow,
) -> AppResult<Versioned<Vec<RefItem>>> {
    with_session(&app, &window, |session| {
        Ok(session.repo().list_branches()?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_tags(
    app: AppHandle,
    window: WebviewWindow,
) -> AppResult<Versioned<Vec<RefItem>>> {
    with_session(&app, &window, |session| Ok(session.repo().list_tags()?)).await
}

/// Commits from `HEAD`, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_commits(
    app: AppHandle,
    window: WebviewWindow,
    offset: u32,
    limit: u32,
) -> AppResult<Versioned<Vec<CommitItem>>> {
    with_session(&app, &window, move |session| {
        Ok(session
            .repo()
            .list_commits(offset as usize, limit as usize)?)
    })
    .await
}

/// Commits whose message contains `query`, ignoring case.
#[tauri::command]
#[specta::specta]
pub async fn search_commits(
    app: AppHandle,
    window: WebviewWindow,
    query: String,
    limit: u32,
) -> AppResult<Versioned<Vec<CommitItem>>> {
    with_session(&app, &window, move |session| {
        Ok(session.repo().search_commits(&query, limit as usize)?)
    })
    .await
}

/// The commits whose SHA starts with `prefix`, for a pasted SHA.
#[tauri::command]
#[specta::specta]
pub async fn resolve_commit_prefix(
    app: AppHandle,
    window: WebviewWindow,
    prefix: String,
) -> AppResult<Versioned<Vec<CommitItem>>> {
    with_session(&app, &window, move |session| {
        Ok(session.repo().resolve_commit_prefix(&prefix)?)
    })
    .await
}

/// Switches this window to `comparison` and lists the changed files. A listing
/// still running for the previous comparison is cancelled.
#[tauri::command]
#[specta::specta]
pub async fn set_comparison(
    app: AppHandle,
    window: WebviewWindow,
    comparison: Comparison,
) -> AppResult<ChangeSet> {
    let state = app.state::<AppState>();
    let session = state.session(window.label())?;
    let algorithm = state.settings().diff_algorithm;
    let ticket = session.begin(comparison);
    blocking(move || session.list_changes(&ticket, algorithm)).await
}

/// The diff of one file from the current list. `force` shows a large file's
/// diff anyway ("Load diff").
#[tauri::command]
#[specta::specta]
pub async fn get_file_diff(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
    force: bool,
) -> AppResult<Versioned<FileDiff>> {
    let state = app.state::<AppState>();
    let session = state.session(window.label())?;
    let settings = state.settings();
    let (generation, file, ignore_whitespace) = session.file(&path)?;
    let options = DiffOptions {
        algorithm: settings.diff_algorithm,
        ignore_whitespace,
        context: settings.context_lines.into(),
        force,
    };
    blocking(move || {
        Ok(Versioned {
            generation,
            value: session.repo().file_diff(&file, &options)?,
        })
    })
    .await
}

/// Lines `start..=end` (1-based) of one side of a file, for expanding context.
#[tauri::command]
#[specta::specta]
pub async fn get_lines(
    app: AppHandle,
    window: WebviewWindow,
    path: String,
    side: Side,
    start: u32,
    end: u32,
) -> AppResult<Versioned<Vec<String>>> {
    let session = app.state::<AppState>().session(window.label())?;
    let (generation, file, _) = session.file(&path)?;
    blocking(move || {
        Ok(Versioned {
            generation,
            value: session.repo().lines(&file, side, start, end)?,
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_settings(app: AppHandle) -> Settings {
    app.state::<AppState>().settings()
}

/// Changes the given settings, saves them and sends `settings-changed` to
/// every window.
#[tauri::command]
#[specta::specta]
pub async fn set_settings(app: AppHandle, patch: SettingsPatch) -> Settings {
    app::update_settings(&app, patch)
}

#[tauri::command]
#[specta::specta]
pub async fn list_recent(app: AppHandle) -> Vec<RecentRepo> {
    app.state::<AppState>().recent()
}

#[tauri::command]
#[specta::specta]
pub async fn remove_recent(app: AppHandle, path: PathBuf) -> Vec<RecentRepo> {
    app::remove_recent(&app, &path)
}

/// Runs `f` with this window's session on the blocking thread pool, and tags
/// the result with the generation from before it started.
async fn with_session<T: Send + 'static>(
    app: &AppHandle,
    window: &WebviewWindow,
    f: impl FnOnce(&Session) -> AppResult<T> + Send + 'static,
) -> AppResult<Versioned<T>> {
    let session: Arc<Session> = app.state::<AppState>().session(window.label())?;
    blocking(move || {
        let generation = session.generation();
        Ok(Versioned {
            generation,
            value: f(&session)?,
        })
    })
    .await
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(AppError::internal)?
}

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            app_version,
            get_window_state,
            open_repo,
            show_open_dialog,
            list_branches,
            list_tags,
            list_commits,
            search_commits,
            resolve_commit_prefix,
            set_comparison,
            get_file_diff,
            get_lines,
            get_settings,
            set_settings,
            list_recent,
            remove_recent,
        ])
        .events(collect_events![
            SettingsChanged,
            RecentChanged,
            RepoOpened,
            OpenFailed
        ])
        // One TypeScript type per Rust type, rather than `X_Serialize` and
        // `X_Deserialize` variants (TECH.md §5).
        .disable_serde_phases()
        // Times (seconds) and file sizes are far below 2^53.
        .dangerously_cast_bigints_to_number()
}

pub fn export_types(
    builder: &Builder<tauri::Wry>,
    path: &str,
) -> Result<(), specta_typescript::Error> {
    builder.export(Typescript::default(), path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_matches_cargo_toml() {
        let version = tauri::async_runtime::block_on(app_version());
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }

    /// Fails when `src/lib/types.ts` doesn't match the Rust types, and rewrites it
    /// so that running the tests again (and committing the file) fixes it.
    #[test]
    fn types_ts_is_up_to_date() {
        let fresh = std::env::temp_dir().join(format!("turbodiff-types-{}.ts", std::process::id()));
        export_types(&specta_builder(), fresh.to_str().unwrap()).unwrap();
        let expected = std::fs::read_to_string(&fresh).unwrap();
        std::fs::remove_file(&fresh).unwrap();

        let current = std::fs::read_to_string(TYPES_PATH).unwrap_or_default();
        if current != expected {
            std::fs::write(TYPES_PATH, &expected).unwrap();
            panic!(
                "src/lib/types.ts was out of date and has been regenerated. Review and commit it."
            );
        }
    }
}
