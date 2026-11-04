//! Read-only repository access with gitoxide and imara-diff (TECH.md §4).
//!
//! This is the only crate that may use `gix`, and it must not depend on Tauri.

mod error;
mod repo;

pub use error::{Error, Result};
pub use repo::{HeadInfo, Repo, RepoInfo};
