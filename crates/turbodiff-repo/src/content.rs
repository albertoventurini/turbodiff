//! Loading file contents from trees and the work tree.

use std::path::Path;

use gix::ObjectId;
use gix::bstr::{BStr, BString, ByteSlice};
use gix::filter::plumbing::pipeline::convert::ToGitOutcome;
use gix::object::tree::EntryKind;

use crate::{Error, Result};

/// git looks for a NUL byte in this many leading bytes to tell binary files.
const BINARY_CHECK_LEN: usize = 8_000;

/// One side of a changed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Version {
    pub path: BString,
    pub kind: EntryKind,
    /// The blob, or the commit of a submodule. For a work-tree file, the id
    /// its filtered content would have.
    pub id: ObjectId,
    /// The content is read from the work tree, not the object database.
    pub on_disk: bool,
}

impl Version {
    pub fn mode(&self) -> String {
        self.kind.as_octal_str().to_string()
    }
}

/// Reads versions, applying the same filters git would when adding
/// work-tree files (TECH.md §4.4).
pub(crate) struct Loader<'repo> {
    repo: &'repo gix::Repository,
    root: &'repo Path,
    pipeline: gix::filter::Pipeline<'repo>,
    index: gix::worktree::IndexPersistedOrInMemory,
    attributes: gix::AttributeStack<'repo>,
    matches: gix::attrs::search::Outcome,
}

impl<'repo> Loader<'repo> {
    pub fn new(repo: &'repo gix::Repository, root: &'repo Path) -> Result<Self> {
        let (pipeline, index) = repo.filter_pipeline(None)?;
        let attributes = repo.attributes_only(
            &index,
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
        )?;
        let matches = attributes.selected_attribute_matches(["diff"]);
        Ok(Self {
            repo,
            root,
            pipeline,
            index,
            attributes,
            matches,
        })
    }

    pub fn index(&self) -> &gix::index::State {
        &self.index
    }

    /// The content of `version`. A submodule reads as git shows it in a diff.
    pub fn load(&mut self, version: &Version) -> Result<Vec<u8>> {
        if version.kind == EntryKind::Commit {
            return Ok(format!("Subproject commit {}\n", version.id).into_bytes());
        }
        if !version.on_disk {
            return Ok(self.repo.find_blob(version.id)?.take_data());
        }
        let path = self
            .root
            .join(gix::path::from_bstr(version.path.as_bstr())?);
        if version.kind == EntryKind::Link {
            let target = std::fs::read_link(&path)?;
            return Ok(gix::path::into_bstr(target)?.into_owned().into());
        }
        self.read_filtered(&path, version.path.as_bstr())
    }

    /// Reads a work-tree file and converts it as `git add` would.
    pub fn read_filtered(&mut self, path: &Path, rela_path: &BStr) -> Result<Vec<u8>> {
        let file = std::fs::File::open(path)?;
        let rela_path = gix::path::from_bstr(rela_path)?;
        let mut out = Vec::new();
        match self
            .pipeline
            .convert_to_git(file, &rela_path, &self.index)?
        {
            ToGitOutcome::Unchanged(mut file) => {
                std::io::copy(&mut file, &mut out)?;
            }
            ToGitOutcome::Buffer(buf) => out.extend_from_slice(buf),
            ToGitOutcome::Process(mut read) => {
                std::io::copy(&mut read, &mut out)?;
            }
        }
        Ok(out)
    }

    /// Whether `.gitattributes` marks `path` as binary (`-diff`, or `binary`,
    /// which implies it).
    pub fn binary_by_attributes(&mut self, path: &BStr, kind: EntryKind) -> Result<bool> {
        let mode = Some(gix::index::entry::Mode::from(kind));
        let platform = self
            .attributes
            .at_entry(path, mode)
            .map_err(Error::internal)?;
        self.matches.reset();
        platform.matching_attributes(&mut self.matches);
        Ok(self
            .matches
            .iter_selected()
            .any(|m| m.assignment.state.is_unset()))
    }
}

/// git's rule for binary content: a NUL byte near the start.
pub(crate) fn has_nul(data: &[u8]) -> bool {
    data[..data.len().min(BINARY_CHECK_LEN)].contains(&0)
}

/// TurboDiff shows content as binary when git would, or when it isn't valid
/// UTF-8 (PRD §8).
pub(crate) fn is_binary(data: &[u8]) -> bool {
    has_nul(data) || std::str::from_utf8(data).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nul_byte_near_the_start_is_binary() {
        assert!(has_nul(b"PNG\0data"));
        assert!(!has_nul(b"plain text\n"));
        let mut late = vec![b'a'; BINARY_CHECK_LEN];
        late.push(0);
        assert!(!has_nul(&late));
    }

    #[test]
    fn invalid_utf8_is_binary() {
        assert!(is_binary(b"caf\xe9\n"));
        assert!(!is_binary("café\n".as_bytes()));
        assert!(!is_binary(b""));
    }
}
