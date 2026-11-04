//! App services: windows, the command line, settings, recent repositories and
//! menus (TECH.md §4.7).

mod boot;
pub mod cli;
pub mod fonts;
pub mod menu;
pub mod placement;
pub mod recent;
mod refresh;
pub mod settings;
mod store;
pub mod windows;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::{RecentChanged, SettingsChanged};
use crate::session::Session;
use crate::watch::Watcher;
use recent::{RecentList, RecentRepo};
use settings::{Settings, SettingsPatch};

/// State shared by all windows.
#[derive(Default)]
pub struct AppState {
    /// Repository windows by label. Other windows, such as Settings, have no
    /// entry.
    windows: Mutex<HashMap<String, WindowSlot>>,
    settings: Mutex<Settings>,
    recent: Mutex<RecentList>,
}

/// What a repository window shows.
#[derive(Default)]
pub struct WindowSlot {
    /// `None` while the window shows the welcome screen.
    pub session: Option<Arc<Session>>,
    /// An error to show when the window's page asks for its state, for a
    /// window opened after opening a folder failed.
    pub pending_error: Option<AppError>,
    /// Watches the session's repository (TECH.md §4.6). Dropping it stops
    /// watching.
    pub watcher: Option<Watcher>,
}

impl AppState {
    pub fn windows(&self) -> MutexGuard<'_, HashMap<String, WindowSlot>> {
        lock(&self.windows)
    }

    pub fn session(&self, label: &str) -> Result<Arc<Session>, AppError> {
        self.windows()
            .get(label)
            .and_then(|slot| slot.session.clone())
            .ok_or(AppError::NoRepository)
    }

    pub fn settings(&self) -> Settings {
        lock(&self.settings).clone()
    }

    pub fn recent(&self) -> Vec<RecentRepo> {
        lock(&self.recent).items(Path::is_dir)
    }
}

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Loads the saved settings and recent list, builds the menu and opens the
/// first window.
pub fn setup(app: &AppHandle, launch: cli::Launch) -> tauri::Result<()> {
    let (settings, recent) = store::load(app);
    app.manage(AppState {
        settings: Mutex::new(settings),
        recent: Mutex::new(recent),
        ..Default::default()
    });
    menu::install(app)?;
    windows::launch(app, launch);
    Ok(())
}

/// Applies `patch`, saves the result and sends it to every window.
pub fn update_settings(app: &AppHandle, patch: SettingsPatch) -> Settings {
    change_settings(app, |settings| settings.apply(patch))
}

/// Restores the defaults, saves them and sends them to every window.
pub fn reset_settings(app: &AppHandle) -> Settings {
    change_settings(app, Settings::reset)
}

fn change_settings(app: &AppHandle, change: impl FnOnce(&mut Settings)) -> Settings {
    let state = app.state::<AppState>();
    let settings = {
        let mut settings = lock(&state.settings);
        change(&mut settings);
        settings.clone()
    };
    store::save_settings(app, &settings);
    if let Err(err) = SettingsChanged(settings.clone()).emit(app) {
        eprintln!("turbodiff: can't send settings-changed: {err}");
    }
    settings
}

/// Moves `root` to the front of the recent list.
pub fn remember_recent(app: &AppHandle, root: &Path) {
    change_recent(app, |recent| {
        recent.push(root);
        true
    });
}

pub fn remove_recent(app: &AppHandle, path: &Path) -> Vec<RecentRepo> {
    change_recent(app, |recent| recent.remove(path))
}

fn change_recent(app: &AppHandle, change: impl FnOnce(&mut RecentList) -> bool) -> Vec<RecentRepo> {
    let state = app.state::<AppState>();
    let changed = {
        let mut recent = lock(&state.recent);
        let changed = change(&mut recent);
        if changed {
            store::save_recent(app, &recent);
        }
        changed
    };
    let items = state.recent();
    if changed {
        menu::set_recent(app, &items);
        if let Err(err) = RecentChanged(items.clone()).emit(app) {
            eprintln!("turbodiff: can't send recent-changed: {err}");
        }
    }
    items
}
