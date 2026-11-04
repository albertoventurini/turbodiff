use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{Error, Result};

/// An open repository. It is only ever read (TECH.md §8).
pub struct Repo {
    inner: gix::ThreadSafeRepository,
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    /// The work-tree root.
    pub root: PathBuf,
    /// The name of the work-tree folder.
    pub name: String,
    pub head: HeadInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HeadInfo {
    /// The branch `HEAD` points to, without `refs/heads/`. `None` when detached.
    pub branch: Option<String>,
    /// The commit `HEAD` points to. `None` when there are no commits yet.
    pub commit: Option<String>,
}

impl Repo {
    /// Opens the repository that contains `path` (PRD §6.1). A path outside any
    /// repository, or a bare repository, is `NotARepository`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_dir() {
            return Err(Error::NotARepository(path.to_owned()));
        }
        let mut repo = gix::discover(path).map_err(|err| {
            if err
                .downcast_any_ref::<gix::discover::upwards::Error>()
                .is_some()
            {
                Error::NotARepository(path.to_owned())
            } else {
                Error::from(err)
            }
        })?;
        let Some(root) = repo.workdir() else {
            return Err(Error::NotARepository(path.to_owned()));
        };
        let root = gix::path::realpath(root).map_err(Error::internal)?;
        strip_filter_drivers(&mut repo)?;
        Ok(Self {
            inner: repo.into_sync(),
            root,
        })
    }

    /// A thread-local handle, for one operation.
    pub(crate) fn local(&self) -> gix::Repository {
        self.inner.to_thread_local()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn info(&self) -> Result<RepoInfo> {
        let repo = self.local();
        let head = repo.head()?;
        let branch = head.referent_name().map(|name| name.shorten().to_string());
        let commit = head.id().map(|id| id.to_string());
        let name = self
            .root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string());
        Ok(RepoInfo {
            root: self.root.clone(),
            name,
            head: HeadInfo { branch, commit },
        })
    }
}

/// Removes `filter.<driver>` sections from the in-memory configuration, so no
/// external clean or smudge command (such as Git LFS) ever runs. Those may
/// write into `.git`. Built-in filters (line endings, `ident`) still apply.
/// The configuration files on disk are untouched.
fn strip_filter_drivers(repo: &mut gix::Repository) -> Result<()> {
    let mut config = repo.config_snapshot_mut();
    let ids: Vec<_> = config
        .sections_and_ids_by_name("filter")
        .map(|sections| sections.map(|(_, id)| id).collect())
        .unwrap_or_default();
    for id in ids {
        config.remove_section_by_id(id);
    }
    config.commit()?;
    Ok(())
}
