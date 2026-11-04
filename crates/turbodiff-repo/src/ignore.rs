//! Which work-tree paths git ignores, so the file watcher can leave them out
//! (TECH.md §4.6).

use std::path::{Component, Path, PathBuf};

use gix::bstr::ByteSlice;
use gix::worktree::stack::state::ignore::Source;

use crate::{Repo, Result};

/// The excludes from `.gitignore` files, `info/exclude` and `core.excludesFile`,
/// as they are when it's created.
struct Excludes<'repo> {
    stack: gix::AttributeStack<'repo>,
    index: gix::worktree::Index,
}

impl<'repo> Excludes<'repo> {
    fn new(repo: &'repo gix::Repository) -> Result<Self> {
        let index = repo.index_or_empty()?;
        let stack = repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)?;
        Ok(Self { stack, index })
    }

    /// Whether git ignores `relative`: it or one of its folders matches an
    /// exclude pattern, and nothing at or below it is tracked.
    fn is_ignored(&mut self, relative: &Path, is_dir: bool) -> Result<bool> {
        let components: Vec<_> = relative.components().collect();
        let mut prefix = PathBuf::new();
        let mut excluded = false;
        for (i, component) in components.iter().enumerate() {
            let Component::Normal(name) = component else {
                continue;
            };
            prefix.push(name);
            let dir = is_dir || i + 1 < components.len();
            let mode = dir.then_some(gix::index::entry::Mode::DIR);
            if self.stack.at_path(&prefix, mode)?.is_excluded() {
                excluded = true;
                break;
            }
        }
        Ok(excluded && !self.is_tracked(relative, is_dir))
    }

    /// Whether the index has `relative`, or for a folder, anything below it.
    fn is_tracked(&self, relative: &Path, is_dir: bool) -> bool {
        let Some(path) = gix::path::os_str_into_bstr(relative.as_os_str()).ok() else {
            return false;
        };
        let path = gix::path::to_unix_separators_on_windows(path);
        if is_dir {
            let mut prefix = path.into_owned();
            prefix.push(b'/');
            self.index.prefixed_entries(prefix.as_bstr()).is_some()
        } else {
            self.index.entry_by_path(path.as_bstr()).is_some()
        }
    }
}

impl Repo {
    /// For each `(path, is_dir)` below the work-tree root, whether git ignores
    /// it. `.gitignore` files are read again on every call, so edits to them
    /// apply. Paths outside the work tree count as not ignored.
    pub fn ignored(&self, paths: &[(&Path, bool)]) -> Result<Vec<bool>> {
        let repo = self.local();
        let mut excludes = Excludes::new(&repo)?;
        paths
            .iter()
            .map(|(path, is_dir)| match path.strip_prefix(self.root()) {
                Ok(relative) => excludes.is_ignored(relative, *is_dir),
                Err(_) => Ok(false),
            })
            .collect()
    }

    /// `dir` and every folder below it that git doesn't ignore, leaving out
    /// `.git`. Folders that can't be read are skipped. Used to watch the work
    /// tree folder by folder.
    pub fn unignored_dirs(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let repo = self.local();
        let mut excludes = Excludes::new(&repo)?;
        let mut dirs = Vec::new();
        let mut pending = vec![dir.to_owned()];
        while let Some(dir) = pending.pop() {
            let Ok(relative) = dir.strip_prefix(self.root()) else {
                continue;
            };
            if relative.starts_with(".git") || excludes.is_ignored(relative, true)? {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    pending.push(entry.path());
                }
            }
            dirs.push(dir);
        }
        dirs.sort();
        Ok(dirs)
    }
}
