//! Repository windows: one repository per window, opened from the menu, the
//! welcome screen, a drop or the command line (PRD §6.1).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{
    AppHandle, DragDropEvent, Manager, WebviewUrl, WebviewWindowBuilder, Window, WindowEvent,
};
use tauri_plugin_dialog::DialogExt;
use tauri_specta::Event;
use turbodiff_repo::{Comparison, Repo};

use super::cli::{self, Launch};
use super::placement::{self, Placement, Target};
use super::{AppState, WindowSlot};
use crate::error::{AppError, AppResult};
use crate::events::{OpenFailed, RepoOpened};
use crate::session::Session;

pub const SETTINGS_LABEL: &str = "settings";

/// Handles this process's own command line.
pub fn launch(app: &AppHandle, launch: Launch) {
    match launch {
        Launch::Welcome => open_welcome(app),
        Launch::Open { path, comparison } => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = open(&app, path, comparison, Target::AnyEmpty).await {
                    // Show the error on the welcome screen.
                    let label = new_label(&app);
                    app.state::<AppState>().windows().insert(
                        label.clone(),
                        WindowSlot {
                            pending_error: Some(err),
                            ..Default::default()
                        },
                    );
                    create(&app, &label, None);
                }
            });
        }
    }
}

/// Handles the command line of a second launch, which
/// `tauri-plugin-single-instance` forwards to this process.
pub fn second_launch(app: &AppHandle, argv: Vec<String>, cwd: String) {
    match cli::parse(argv, Path::new(&cwd)) {
        Ok(Launch::Welcome) => {
            let empty = app
                .state::<AppState>()
                .windows()
                .iter()
                .find(|(_, slot)| slot.session.is_none())
                .map(|(label, _)| label.clone());
            match empty {
                Some(label) => focus(app, &label),
                None => open_welcome(app),
            }
        }
        Ok(Launch::Open { path, comparison }) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = open(&app, path, comparison, Target::AnyEmpty).await {
                    report(&app, focused(&app).as_deref(), error);
                }
            });
        }
        // The second process has exited, so there's nowhere to print the
        // usage error. Bring the app forward instead.
        Err(_) => {
            if let Some(label) = focused(app).or_else(|| app.webview_windows().into_keys().next()) {
                focus(app, &label);
            }
        }
    }
}

/// Opens the repository containing `path`: focuses the window that already
/// shows it, or shows it in the target window if that's empty, or in a new
/// window.
pub async fn open(
    app: &AppHandle,
    path: PathBuf,
    comparison: Option<Comparison>,
    target: Target,
) -> AppResult<()> {
    let repo = tauri::async_runtime::spawn_blocking(move || Repo::open(&path))
        .await
        .map_err(AppError::internal)??;
    let root = repo.root().to_owned();
    let state = app.state::<AppState>();

    // Decide and claim the window under one lock, so two opens can't pick the
    // same window. Windows are created and events sent after unlocking.
    let (label, placement, session) = {
        let mut windows = state.windows();
        let open: Vec<_> = windows
            .iter()
            .map(|(label, slot)| (label.as_str(), slot.session.as_ref().map(|s| s.root())))
            .collect();
        let placement = placement::place(&open, &root, &target);
        let label = match &placement {
            Placement::Focus(label) | Placement::Reuse(label) => label.clone(),
            Placement::NewWindow => {
                let mut taken: Vec<String> = windows.keys().cloned().collect();
                taken.extend(app.webview_windows().into_keys());
                placement::next_label(taken.iter().map(String::as_str))
            }
        };
        let session = match placement {
            Placement::Focus(_) => windows[&label].session.clone(),
            _ => {
                let session = Arc::new(Session::new(repo, comparison.clone().unwrap_or_default()));
                windows.insert(
                    label.clone(),
                    WindowSlot {
                        session: Some(session.clone()),
                        pending_error: None,
                    },
                );
                Some(session)
            }
        };
        (label, placement, session)
    };

    let title = title(&root);
    match placement {
        Placement::Focus(_) => {
            focus(app, &label);
            // A launch that names refs shows them in the open window.
            if let (Some(comparison), Some(session)) = (comparison, session) {
                emit_opened(app, &label, &session, comparison);
            }
            return Ok(());
        }
        Placement::Reuse(_) => {
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.set_title(&title);
                let _ = window.set_focus();
            }
            if let Some(session) = session {
                let comparison = session.comparison();
                emit_opened(app, &label, &session, comparison);
            }
        }
        Placement::NewWindow => create(app, &label, Some(&title)),
    }
    super::remember_recent(app, &root);
    Ok(())
}

/// Starts [`open`] for a request that has no caller waiting for the result,
/// and shows an error in the window it came from.
pub fn open_in_background(app: &AppHandle, path: PathBuf, target: Target) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let label = match &target {
            Target::Window(label) => Some(label.clone()),
            Target::AnyEmpty => focused(&app),
        };
        if let Err(error) = open(&app, path, None, target).await {
            report(&app, label.as_deref(), error);
        }
    });
}

/// Shows the native folder picker, then opens the chosen folder. `from` is the
/// window that asked, if any.
pub fn choose_folder(app: &AppHandle, from: Option<String>) {
    let window = from
        .as_deref()
        .and_then(|label| app.get_webview_window(label));
    let mut dialog = app.dialog().file().set_title("Open Repository");
    if let Some(window) = &window {
        dialog = dialog.set_parent(window);
    }
    let app = app.clone();
    dialog.pick_folder(move |folder| {
        let Some(path) = folder.and_then(|folder| folder.into_path().ok()) else {
            return;
        };
        let target = from.map_or(Target::AnyEmpty, Target::Window);
        open_in_background(&app, path, target);
    });
}

fn emit_opened(app: &AppHandle, label: &str, session: &Session, comparison: Comparison) {
    let event = session
        .repo()
        .info()
        .map(|repo| RepoOpened { repo, comparison });
    let result = match event {
        Ok(event) => event.emit_to(app, label).map_err(AppError::from),
        Err(err) => OpenFailed { error: err.into() }
            .emit_to(app, label)
            .map_err(AppError::from),
    };
    if let Err(err) = result {
        eprintln!("turbodiff: can't send repo-opened: {err}");
    }
}

/// Shows `error` in window `label`, or in some open window, brought forward.
fn report(app: &AppHandle, label: Option<&str>, error: AppError) {
    let label = label
        .map(str::to_owned)
        .filter(|label| app.get_webview_window(label).is_some())
        .or_else(|| app.webview_windows().into_keys().min());
    let Some(label) = label else {
        eprintln!("turbodiff: {error}");
        return;
    };
    focus(app, &label);
    if let Err(err) = (OpenFailed { error }).emit_to(app, &label) {
        eprintln!("turbodiff: can't send open-failed: {err}");
    }
}

/// Opens an empty window with the welcome screen.
pub fn open_welcome(app: &AppHandle) {
    let label = new_label(app);
    app.state::<AppState>()
        .windows()
        .insert(label.clone(), WindowSlot::default());
    create(app, &label, None);
}

/// Opens the Settings window, or focuses it.
pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.set_focus();
        return;
    }
    let url = WebviewUrl::App("index.html?window=settings".into());
    let result = WebviewWindowBuilder::new(app, SETTINGS_LABEL, url)
        .title("Settings")
        .inner_size(640.0, 560.0)
        .min_inner_size(480.0, 400.0)
        .build();
    if let Err(err) = result {
        eprintln!("turbodiff: can't open the Settings window: {err}");
    }
}

/// The label for a new repository window.
fn new_label(app: &AppHandle) -> String {
    let mut taken: Vec<String> = app.webview_windows().into_keys().collect();
    taken.extend(app.state::<AppState>().windows().keys().cloned());
    placement::next_label(taken.iter().map(String::as_str))
}

/// Creates repository window `label`, whose slot is already set. Size and
/// position are restored by `tauri-plugin-window-state`.
fn create(app: &AppHandle, label: &str, title: Option<&str>) {
    let result = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(title.unwrap_or("TurboDiff"))
        .inner_size(1200.0, 800.0)
        .min_inner_size(640.0, 400.0)
        .build();
    if let Err(err) = result {
        eprintln!("turbodiff: can't open a window: {err}");
        app.state::<AppState>().windows().remove(label);
    }
}

fn title(root: &Path) -> String {
    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    format!("{name} — TurboDiff")
}

fn focus(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// The focused window's label.
pub fn focused(app: &AppHandle) -> Option<String> {
    app.webview_windows()
        .into_iter()
        .find(|(_, window)| window.is_focused().unwrap_or(false))
        .map(|(label, _)| label)
}

/// Opening a dropped folder, and ending a window's session when it closes.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    match event {
        // The first dropped path that is a folder gets opened (TECH.md §4.7).
        WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) => {
            if let Some(folder) = paths.iter().find(|path| path.is_dir()) {
                let target = Target::Window(window.label().to_owned());
                open_in_background(app, folder.clone(), target);
            }
        }
        WindowEvent::Destroyed => {
            let slot = app.state::<AppState>().windows().remove(window.label());
            if let Some(session) = slot.and_then(|slot| slot.session) {
                session.close();
            }
        }
        _ => {}
    }
}
