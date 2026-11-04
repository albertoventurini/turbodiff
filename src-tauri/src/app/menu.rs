//! Native menus (PRD §6.10, TECH.md §4.7). The only accelerator is Quit's
//! `Mod+Q`; on macOS the Edit menu also has the system's Copy and Select All,
//! which the webview needs for the clipboard.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Wry};

use super::placement::Target;
use super::recent::RecentRepo;
use super::{AppState, lock, windows};

const OPEN: &str = "open";
const CLOSE: &str = "close";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";
/// Followed by the repository path.
const RECENT_PREFIX: &str = "recent:";

/// The Open Recent submenu, rebuilt when the list changes.
struct RecentMenu(Mutex<Submenu<Wry>>);

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, OPEN, "Open…", true, None::<&str>)?;
    let recent = Submenu::new(app, "Open Recent", true)?;
    let close = MenuItem::with_id(app, CLOSE, "Close Window", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS, "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit TurboDiff", true, Some("CmdOrCtrl+Q"))?;
    let separator = || PredefinedMenuItem::separator(app);

    let menu = if cfg!(target_os = "macos") {
        let app_menu =
            Submenu::with_items(app, "TurboDiff", true, &[&settings, &separator()?, &quit])?;
        let file =
            Submenu::with_items(app, "File", true, &[&open, &recent, &separator()?, &close])?;
        let edit = Submenu::with_items(
            app,
            "Edit",
            true,
            &[
                &PredefinedMenuItem::copy(app, None)?,
                &PredefinedMenuItem::select_all(app, None)?,
            ],
        )?;
        Menu::with_items(app, &[&app_menu, &file, &edit])?
    } else {
        // Linux has no app menu, so Settings and Quit go in File.
        let file = Submenu::with_items(
            app,
            "File",
            true,
            &[
                &open,
                &recent,
                &separator()?,
                &settings,
                &separator()?,
                &close,
                &quit,
            ],
        )?;
        Menu::with_items(app, &[&file])?
    };
    app.set_menu(menu)?;
    app.manage(RecentMenu(Mutex::new(recent)));
    set_recent(app, &app.state::<AppState>().recent());
    Ok(())
}

/// Replaces the Open Recent items. Unavailable repositories are disabled.
pub fn set_recent(app: &AppHandle, items: &[RecentRepo]) {
    let Some(menu) = app.try_state::<RecentMenu>() else {
        return;
    };
    let submenu = lock(&menu.0);
    let result = (|| {
        for item in submenu.items()? {
            submenu.remove(&item)?;
        }
        for item in items {
            let id = format!("{RECENT_PREFIX}{}", item.path.display());
            let label = item.path.display().to_string();
            submenu.append(&MenuItem::with_id(
                app,
                id,
                label,
                item.available,
                None::<&str>,
            )?)?;
        }
        submenu.set_enabled(!items.is_empty())
    })();
    if let Err(err) = result {
        eprintln!("turbodiff: can't update Open Recent: {err}");
    }
}

pub fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let focused = windows::focused(app);
    match event.id().as_ref() {
        OPEN => windows::choose_folder(app, focused),
        CLOSE => {
            if let Some(window) = focused.and_then(|label| app.get_webview_window(&label)) {
                let _ = window.close();
            }
        }
        SETTINGS => windows::open_settings(app),
        QUIT => app.exit(0),
        id => {
            if let Some(path) = id.strip_prefix(RECENT_PREFIX) {
                let target = focused.map_or(Target::AnyEmpty, Target::Window);
                windows::open_in_background(app, PathBuf::from(path), target);
            }
        }
    }
}
