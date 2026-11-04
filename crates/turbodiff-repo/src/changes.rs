//! Listing changed files with their `+/−` counts (TECH.md §4.4).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use gix::ObjectId;
use gix::bstr::{BString, ByteSlice};
use gix::object::tree::EntryKind;
use gix::status::index_worktree;
use serde::Serialize;

use crate::compare::{Comparison, Resolved, Right};
use crate::content::{self, Loader, Version};
use crate::linediff::{self, Algorithm};
use crate::renames::{self, Candidate, Content, Which};
use crate::{Error, Repo, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
pub enum Status {
    #[serde(rename = "A")]
    Added,
    #[serde(rename = "M")]
    Modified,
    #[serde(rename = "D")]
    Deleted,
    #[serde(rename = "R")]
    Renamed,
    #[serde(rename = "T")]
    TypeChanged,
}

/// One changed file, as listed in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    /// The path before a rename.
    pub old_path: Option<String>,
    pub status: Status,
    /// For renames, how similar the two versions are, in percent.
    pub similarity: Option<u8>,
    pub additions: u32,
    pub deletions: u32,
    /// Binary files have no line counts.
    pub binary: bool,
    /// The old and new modes, such as `100644` and `100755`, when they differ.
    pub mode_change: Option<(String, String)>,
    /// The old and new commits of a submodule. A side that doesn't exist is
    /// all zeros.
    pub submodule: Option<(String, String)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub files: u32,
    pub additions: u32,
    pub deletions: u32,
}

/// A changed file and the versions it was computed from.
#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub entry: FileEntry,
    pub(crate) old: Option<Version>,
    pub(crate) new: Option<Version>,
}

/// The changed files of a comparison, sorted by path.
#[derive(Debug, Clone)]
pub struct Changes {
    pub files: Vec<ChangedFile>,
    pub totals: Totals,
    /// See [`Resolved::no_merge_base`].
    pub no_merge_base: bool,
}

impl Changes {
    pub fn entries(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.iter().map(|file| &file.entry)
    }
}

/// A path that differs, before rename detection.
struct RawChange {
    old: Option<Version>,
    new: Option<Version>,
}

impl Repo {
    /// Resolves `comparison` and lists what changed, with line counts from
    /// `algorithm` (PRD §6.8).
    pub fn changes(&self, comparison: &Comparison, algorithm: Algorithm) -> Result<Changes> {
        let resolved = self.resolve(comparison)?;
        self.changes_for(&resolved, algorithm, comparison.ignore_whitespace)
    }

    pub fn changes_for(
        &self,
        resolved: &Resolved,
        algorithm: Algorithm,
        ignore_whitespace: bool,
    ) -> Result<Changes> {
        let repo = self.local();
        let mut loader = Loader::new(&repo, self.root())?;
        let raw = match resolved.right {
            Right::Tree(right) => tree_changes(&repo, resolved.left, right)?,
            Right::WorkTree => worktree_changes(&repo, self.root(), resolved.left, &mut loader)?,
        };
        let mut files = pair_renames(raw, &mut loader)?
            .into_iter()
            .map(|(old, new, similarity)| {
                let entry = entry(
                    &mut loader,
                    &old,
                    &new,
                    similarity,
                    algorithm,
                    ignore_whitespace,
                )?;
                Ok(ChangedFile { entry, old, new })
            })
            .collect::<Result<Vec<_>>>()?;
        files.sort_by(|a, b| a.entry.path.cmp(&b.entry.path));
        let totals = files.iter().fold(Totals::default(), |totals, file| Totals {
            files: totals.files + 1,
            additions: totals.additions + file.entry.additions,
            deletions: totals.deletions + file.entry.deletions,
        });
        Ok(Changes {
            files,
            totals,
            no_merge_base: resolved.no_merge_base,
        })
    }
}

/// Commit vs commit: a tree-to-tree diff. Renames are found afterwards.
fn tree_changes(repo: &gix::Repository, left: ObjectId, right: ObjectId) -> Result<Vec<RawChange>> {
    use gix::object::tree::diff::ChangeDetached as Change;

    let old_tree = repo.find_tree(left)?;
    let new_tree = repo.find_tree(right)?;
    let mut options = gix::diff::Options::default().with_rewrites(None);
    options.track_path();
    let version = |path: &BString, mode: gix::object::tree::EntryMode, id: ObjectId| {
        (!mode.is_tree()).then(|| Version {
            path: path.clone(),
            kind: mode.kind(),
            id,
            on_disk: false,
        })
    };
    let mut raw = Vec::new();
    for change in repo.diff_tree_to_tree(&old_tree, &new_tree, options)? {
        let (old, new) = match &change {
            Change::Addition {
                location,
                entry_mode,
                id,
                ..
            } => (None, version(location, *entry_mode, *id)),
            Change::Deletion {
                location,
                entry_mode,
                id,
                ..
            } => (version(location, *entry_mode, *id), None),
            Change::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => (
                version(location, *previous_entry_mode, *previous_id),
                version(location, *entry_mode, *id),
            ),
            Change::Rewrite { .. } => {
                return Err(Error::Internal("unexpected rewrite in tree diff".into()));
            }
        };
        if old.is_some() || new.is_some() {
            raw.push(RawChange { old, new });
        }
    }
    Ok(raw)
}

/// Tree vs work tree. Status gives the paths that may differ (tree vs index,
/// index vs work tree, and untracked files), then each one's tree blob is
/// compared directly with the file on disk.
fn worktree_changes(
    repo: &gix::Repository,
    root: &std::path::Path,
    left: ObjectId,
    loader: &mut Loader<'_>,
) -> Result<Vec<RawChange>> {
    let mut paths = BTreeSet::new();
    let mut untracked = HashSet::new();
    let status = repo
        .status(gix::progress::Discard)?
        .head_tree(left)
        .untracked_files(gix::status::UntrackedFiles::Files)
        .index_worktree_rewrites(None)
        .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
        .into_iter(None)?;
    for item in status {
        let item = item?;
        if let gix::status::Item::IndexWorktree(index_worktree::Item::DirectoryContents {
            entry,
            ..
        }) = &item
        {
            let file_like = matches!(
                entry.disk_kind,
                Some(gix::dir::entry::Kind::File | gix::dir::entry::Kind::Symlink)
            );
            if entry.status != gix::dir::entry::Status::Untracked || !file_like {
                continue;
            }
            untracked.insert(entry.rela_path.clone());
        }
        paths.insert(item.location().to_owned());
    }

    let left_tree = repo.find_tree(left)?;
    let file_mode = repo
        .config_snapshot()
        .boolean("core.fileMode")
        .unwrap_or(true);
    let mut raw = Vec::new();
    for path in paths {
        let old = left_tree
            .lookup_entry(path.split(|&b| b == b'/'))?
            .filter(|entry| !entry.mode().is_tree())
            .map(|entry| Version {
                path: path.clone(),
                kind: entry.mode().kind(),
                id: entry.object_id(),
                on_disk: false,
            });
        let tracked = loader.index().entry_by_path(path.as_bstr()).cloned();
        let new = if tracked.is_some() || untracked.contains(&path) {
            worktree_version(repo, root, &path, tracked.as_ref(), file_mode, loader)?
        } else {
            None
        };
        let unchanged = match (&old, &new) {
            (Some(old), Some(new)) => old.id == new.id && old.kind == new.kind,
            (None, None) => true,
            _ => false,
        };
        if !unchanged {
            raw.push(RawChange { old, new });
        }
    }
    Ok(raw)
}

/// The version of `path` in the work tree, or `None` if it isn't there.
fn worktree_version(
    repo: &gix::Repository,
    root: &std::path::Path,
    path: &BString,
    tracked: Option<&gix::index::Entry>,
    file_mode: bool,
    loader: &mut Loader<'_>,
) -> Result<Option<Version>> {
    let full_path = root.join(gix::path::from_bstr(path.as_bstr())?);
    let from_index = |entry: &gix::index::Entry| Version {
        path: path.clone(),
        kind: entry
            .mode
            .to_tree_entry_mode()
            .map_or(EntryKind::Blob, |mode| mode.kind()),
        id: entry.id,
        on_disk: false,
    };
    let metadata = match std::fs::symlink_metadata(&full_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // Sparse checkouts leave skip-worktree entries off disk.
            return Ok(tracked
                .filter(|entry| {
                    entry
                        .flags
                        .contains(gix::index::entry::Flags::SKIP_WORKTREE)
                })
                .map(from_index));
        }
        Err(err) => return Err(err.into()),
    };
    let hash = |data: &[u8]| -> Result<ObjectId> {
        gix::objs::compute_hash(repo.object_hash(), gix::object::Kind::Blob, data)
            .map_err(Error::internal)
    };
    let (kind, id) = if metadata.is_symlink() {
        let target = std::fs::read_link(&full_path)?;
        (EntryKind::Link, hash(&gix::path::into_bstr(target)?)?)
    } else if metadata.is_file() {
        let executable = if file_mode {
            gix::fs::is_executable(&metadata)
        } else {
            tracked.is_some_and(|entry| entry.mode == gix::index::entry::Mode::FILE_EXECUTABLE)
        };
        let kind = if executable {
            EntryKind::BlobExecutable
        } else {
            EntryKind::Blob
        };
        (
            kind,
            hash(&loader.read_filtered(&full_path, path.as_bstr())?)?,
        )
    } else if metadata.is_dir() {
        // Only a submodule can be a directory here. One that isn't checked
        // out is unchanged.
        let Some(entry) = tracked.filter(|entry| entry.mode == gix::index::entry::Mode::COMMIT)
        else {
            return Ok(None);
        };
        let head = gix::open(&full_path)
            .ok()
            .and_then(|submodule| submodule.head_id().ok().map(|id| id.detach()));
        match head {
            Some(head) => (EntryKind::Commit, head),
            None => return Ok(Some(from_index(entry))),
        }
    } else {
        return Ok(None);
    };
    Ok(Some(Version {
        path: path.clone(),
        kind,
        id,
        on_disk: kind != EntryKind::Commit,
    }))
}

fn is_regular(kind: EntryKind) -> bool {
    matches!(kind, EntryKind::Blob | EntryKind::BlobExecutable)
}

fn candidates(versions: &[Version]) -> Vec<Candidate<'_>> {
    versions
        .iter()
        .map(|version| Candidate {
            path: version.path.as_bstr(),
            id: version.id,
            regular: is_regular(version.kind),
        })
        .collect()
}

/// Turns additions and deletions into renames where they match, and returns
/// `(old, new, similarity)` for every file.
#[allow(clippy::type_complexity)]
fn pair_renames(
    raw: Vec<RawChange>,
    loader: &mut Loader<'_>,
) -> Result<Vec<(Option<Version>, Option<Version>, Option<u8>)>> {
    let mut out = Vec::new();
    // Sorted by path, as git sorts its candidates.
    let mut deleted = BTreeMap::new();
    let mut added = BTreeMap::new();
    for change in raw {
        match (change.old, change.new) {
            (Some(old), None) if old.kind != EntryKind::Commit => {
                deleted.insert(old.path.clone(), old);
            }
            (None, Some(new)) if new.kind != EntryKind::Commit => {
                added.insert(new.path.clone(), new);
            }
            (old, new) => out.push((old, new, None)),
        }
    }
    let deleted: Vec<Version> = deleted.into_values().collect();
    let added: Vec<Version> = added.into_values().collect();
    let renames = renames::detect(&candidates(&deleted), &candidates(&added), |which, i| {
        let version = match which {
            Which::Source => &deleted[i],
            Which::Dest => &added[i],
        };
        let data = loader.load(version)?;
        let binary = content::has_nul(&data)
            || loader.binary_by_attributes(version.path.as_bstr(), version.kind)?;
        Ok(Content { data, binary })
    })?;

    let mut source_used = vec![false; deleted.len()];
    let mut dest_used = vec![false; added.len()];
    for rename in &renames {
        source_used[rename.source] = true;
        dest_used[rename.dest] = true;
        out.push((
            Some(deleted[rename.source].clone()),
            Some(added[rename.dest].clone()),
            Some(rename.percent()),
        ));
    }
    for (version, used) in deleted.into_iter().zip(source_used) {
        if !used {
            out.push((Some(version), None, None));
        }
    }
    for (version, used) in added.into_iter().zip(dest_used) {
        if !used {
            out.push((None, Some(version), None));
        }
    }
    Ok(out)
}

/// The sidebar entry for one file, with its line counts.
fn entry(
    loader: &mut Loader<'_>,
    old: &Option<Version>,
    new: &Option<Version>,
    similarity: Option<u8>,
    algorithm: Algorithm,
    ignore_whitespace: bool,
) -> Result<FileEntry> {
    let type_class = |kind: EntryKind| match kind {
        EntryKind::Blob | EntryKind::BlobExecutable => 0,
        EntryKind::Link => 1,
        EntryKind::Commit => 2,
        EntryKind::Tree => 3,
    };
    let status = match (old, new) {
        (Some(_), Some(_)) if similarity.is_some() => Status::Renamed,
        (Some(old), Some(new)) if type_class(old.kind) != type_class(new.kind) => {
            Status::TypeChanged
        }
        (Some(_), Some(_)) => Status::Modified,
        (None, Some(_)) => Status::Added,
        (Some(_), None) => Status::Deleted,
        (None, None) => unreachable!("a change has at least one side"),
    };
    let path = new.as_ref().or(old.as_ref()).expect("a change has a side");
    let mode_change = match (old, new) {
        (Some(old), Some(new)) if old.kind != new.kind => Some((old.mode(), new.mode())),
        _ => None,
    };
    let is_submodule = |version: &Option<Version>| {
        version
            .as_ref()
            .is_some_and(|version| version.kind == EntryKind::Commit)
    };
    let submodule = (is_submodule(old) || is_submodule(new)).then(|| {
        let sha = |version: &Option<Version>| match version {
            Some(version) if version.kind == EntryKind::Commit => version.id.to_string(),
            _ => ObjectId::null(gix::hash::Kind::Sha1).to_string(),
        };
        (sha(old), sha(new))
    });

    let mut binary = false;
    let mut data = [Vec::new(), Vec::new()];
    for (slot, version) in data.iter_mut().zip([old, new]) {
        if let Some(version) = version {
            *slot = loader.load(version)?;
            binary |= content::is_binary(slot)
                || loader.binary_by_attributes(version.path.as_bstr(), version.kind)?;
        }
    }
    let (additions, deletions) = if binary {
        (0, 0)
    } else {
        let old_lines = linediff::split_lines(&data[0]);
        let new_lines = linediff::split_lines(&data[1]);
        linediff::count(&linediff::diff_lines(
            &old_lines,
            &new_lines,
            algorithm,
            ignore_whitespace,
        ))
    };
    Ok(FileEntry {
        path: path.path.to_str_lossy().into_owned(),
        old_path: similarity.and(old.as_ref().map(|old| old.path.to_str_lossy().into_owned())),
        status,
        similarity,
        additions,
        deletions,
        binary,
        mode_change,
        submodule,
    })
}
