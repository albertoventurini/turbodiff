//! Saving settings and recent repositories with `tauri-plugin-store`
//! (TECH.md §4.7), in `settings.json` in the platform config directory.

use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use super::recent::RecentList;
use super::settings::Settings;

const FILE: &str = "settings.json";
const SETTINGS: &str = "settings";
const RECENT: &str = "recent";

/// The saved settings and recent list. Anything unreadable gets its default.
pub fn load(app: &AppHandle) -> (Settings, RecentList) {
    match app.store(FILE) {
        Ok(store) => (
            Settings::from_stored(store.get(SETTINGS).as_ref()),
            RecentList::from_stored(store.get(RECENT).as_ref()),
        ),
        Err(err) => {
            eprintln!("turbodiff: can't read {FILE}: {err}");
            Default::default()
        }
    }
}

pub fn save_settings(app: &AppHandle, settings: &Settings) {
    match serde_json::to_value(settings) {
        Ok(value) => save(app, SETTINGS, value),
        Err(err) => eprintln!("turbodiff: can't save settings: {err}"),
    }
}

pub fn save_recent(app: &AppHandle, recent: &RecentList) {
    save(app, RECENT, recent.to_stored());
}

fn save(app: &AppHandle, key: &str, value: Value) {
    let result = app.store(FILE).and_then(|store| {
        store.set(key, value);
        store.save()
    });
    if let Err(err) = result {
        eprintln!("turbodiff: can't save {FILE}: {err}");
    }
}
