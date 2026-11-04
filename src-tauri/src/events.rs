//! Events sent to the webview (TECH.md §5). Each one's name is its type name
//! in kebab case, such as `settings-changed`.

use serde::Serialize;
use tauri_specta::Event;
use turbodiff_repo::{Comparison, RepoInfo};

use crate::app::recent::RecentRepo;
use crate::app::settings::Settings;
use crate::error::AppError;

/// Sent to every window when the settings change.
#[derive(Debug, Clone, Serialize, specta::Type, Event)]
pub struct SettingsChanged(pub Settings);

/// Sent to every window when the recent repositories list changes.
#[derive(Debug, Clone, Serialize, specta::Type, Event)]
pub struct RecentChanged(pub Vec<RecentRepo>);

/// Sent to a window when it starts showing a repository (from the menu, a
/// drop or the command line), or when a launch asks an open repository for
/// another comparison. A new window gets this from `get_window_state` instead.
#[derive(Debug, Clone, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct RepoOpened {
    pub repo: RepoInfo,
    /// The comparison to show first: work tree vs `HEAD`, unless the command
    /// line chose another (PRD §6.2).
    pub comparison: Comparison,
}

/// Sent to a window when opening a folder from the menu, a drop or the command
/// line failed. The window stays as it was (PRD §6.1).
#[derive(Debug, Clone, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct OpenFailed {
    pub error: AppError,
}
