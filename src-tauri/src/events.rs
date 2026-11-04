//! Events sent to the webview (TECH.md §5). Each one's name is its type name
//! in kebab case, such as `settings-changed`.

use serde::Serialize;
use tauri_specta::Event;
use turbodiff_repo::{Comparison, HeadInfo, RepoInfo};

use crate::app::recent::RecentRepo;
use crate::app::settings::Settings;
use crate::error::AppError;
use crate::session::ChangeSet;

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

/// Sent to a window after its repository changed on disk and the file list
/// was rebuilt (TECH.md §4.6). `files_changed` are the paths whose diffs may
/// differ from the previous list, including ones that left it.
#[derive(Debug, Clone, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct RepoChanged {
    pub changes: ChangeSet,
    pub files_changed: Vec<String>,
    /// `HEAD` now, which moves on a commit or a branch switch.
    pub head: HeadInfo,
}

/// Sent to a window when a refresh found that a ref in its comparison no
/// longer resolves, such as a deleted branch. The window falls back to work
/// tree vs `HEAD` (PRD §6.9).
#[derive(Debug, Clone, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonInvalid {
    /// The ref that no longer resolves.
    pub name: String,
}
