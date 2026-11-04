//! Read-only repository access with gitoxide and imara-diff (TECH.md §4).
//!
//! This is the only crate that may use `gix`, and it must not depend on Tauri.

mod error;
#[allow(dead_code)]
mod linediff;
mod refs;
#[allow(dead_code)]
mod renames;
mod repo;

pub use error::{Error, Result};
pub use linediff::Algorithm;
pub use refs::{CommitItem, RefItem, RefKind, SEARCH_COMMITS_SCAN_LIMIT};
pub use repo::{HeadInfo, Repo, RepoInfo};
