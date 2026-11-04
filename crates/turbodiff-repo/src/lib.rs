//! Read-only repository access with gitoxide and imara-diff (TECH.md §4).
//!
//! This is the only crate that may use `gix`, and it must not depend on Tauri.

mod error;
mod refs;
mod repo;

pub use error::{Error, Result};
pub use refs::{CommitItem, RefItem, RefKind, SEARCH_COMMITS_SCAN_LIMIT};
pub use repo::{HeadInfo, Repo, RepoInfo};
