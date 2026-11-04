//! Resolving a comparison to the trees to diff (TECH.md §4.3).

use gix::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{Error, Repo, Result};

/// What the compare side shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CompareSide {
    /// The files on disk, including untracked ones.
    WorkTree,
    /// A ref or commit: a full ref name, a SHA, or any revision git understands.
    Commit { spec: String },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Compare against the merge base of base and compare (PRD §6.3).
    #[default]
    ThreeDot,
    /// Compare against base as it is now.
    TwoDot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    /// A ref or commit, like [`CompareSide::Commit`].
    pub base: String,
    pub compare: CompareSide,
    pub mode: Mode,
    /// Hide whitespace (PRD §6.7).
    pub ignore_whitespace: bool,
}

impl Default for Comparison {
    /// Working tree vs `HEAD`, which every repository opens with (PRD §6.2).
    fn default() -> Self {
        Self {
            base: "HEAD".into(),
            compare: CompareSide::WorkTree,
            mode: Mode::ThreeDot,
            ignore_whitespace: false,
        }
    }
}

/// The trees a comparison resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The left side, always a tree.
    pub(crate) left: ObjectId,
    pub(crate) right: Right,
    /// Three-dot was asked for but the two sides have no merge base, so this
    /// compares two-dot instead and the UI shows a notice.
    pub no_merge_base: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Right {
    WorkTree,
    Tree(ObjectId),
}

impl Repo {
    /// Resolves the refs in `comparison`. A ref that doesn't exist is
    /// `RefNotFound`. `HEAD` without commits is the empty tree.
    pub fn resolve(&self, comparison: &Comparison) -> Result<Resolved> {
        let repo = self.local();
        let base = resolve_commit(&repo, &comparison.base)?;
        let (right, other) = match &comparison.compare {
            CompareSide::WorkTree => (Right::WorkTree, resolve_commit(&repo, "HEAD")?),
            CompareSide::Commit { spec } => {
                let commit = resolve_commit(&repo, spec)?;
                let tree = match commit {
                    Some(commit) => tree_of(&repo, commit)?,
                    None => empty_tree(&repo),
                };
                (Right::Tree(tree), commit)
            }
        };
        let mut no_merge_base = false;
        let left_commit = match (comparison.mode, base, other) {
            (Mode::ThreeDot, Some(base), Some(other)) => match repo.merge_base(base, other) {
                Ok(Some(merge_base)) => Some(merge_base.detach()),
                Ok(None) => {
                    no_merge_base = true;
                    Some(base)
                }
                Err(err) => return Err(err.into()),
            },
            _ => base,
        };
        let left = match left_commit {
            Some(commit) => tree_of(&repo, commit)?,
            None => empty_tree(&repo),
        };
        Ok(Resolved {
            left,
            right,
            no_merge_base,
        })
    }
}

/// The commit `spec` points to, or `None` for `HEAD` without commits.
fn resolve_commit(repo: &gix::Repository, spec: &str) -> Result<Option<ObjectId>> {
    if spec == "HEAD" && repo.head()?.id().is_none() {
        return Ok(None);
    }
    let not_found = || Error::RefNotFound(spec.to_owned());
    let id = repo.rev_parse_single(spec).map_err(|_| not_found())?;
    let commit = id.object()?.peel_to_commit().map_err(|_| not_found())?;
    Ok(Some(commit.id))
}

fn tree_of(repo: &gix::Repository, commit: ObjectId) -> Result<ObjectId> {
    Ok(repo.find_commit(commit)?.tree_id()?.detach())
}

fn empty_tree(repo: &gix::Repository) -> ObjectId {
    ObjectId::empty_tree(repo.object_hash())
}
