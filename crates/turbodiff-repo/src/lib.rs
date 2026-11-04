//! Read-only repository access with gitoxide and imara-diff (TECH.md §4).
//!
//! This is the only crate that may use `gix`, and it must not depend on Tauri.

mod changes;
mod compare;
mod content;
mod error;
mod linediff;
mod refs;
mod renames;
mod repo;

pub use changes::{ChangedFile, Changes, FileEntry, Status, Totals};
pub use compare::{CompareSide, Comparison, Mode, Resolved};
pub use error::{Error, Result};
pub use linediff::Algorithm;
pub use refs::{CommitItem, RefItem, RefKind, SEARCH_COMMITS_SCAN_LIMIT};
pub use repo::{HeadInfo, Repo, RepoInfo};
