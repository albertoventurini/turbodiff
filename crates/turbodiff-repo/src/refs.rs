//! Branches, tags and commits for the ref picker (TECH.md §4.2).

use std::collections::HashSet;

use gix::bstr::ByteSlice;
use gix::refs::Category;
use gix::revision::walk::Sorting;
use gix::traverse::commit::simple::CommitTimeOrder;
use serde::Serialize;

use crate::{Error, Repo, Result};

/// `search_commits` gives up after looking at this many commits.
pub const SEARCH_COMMITS_SCAN_LIMIT: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RefItem {
    /// The short name, such as `main`, `origin/main` or `v1.0`.
    pub name: String,
    /// The full name, such as `refs/heads/main`.
    pub full_name: String,
    pub kind: RefKind,
    /// The commit the ref points to, after peeling tags.
    pub commit: String,
    /// Whether this is the branch `HEAD` points to.
    pub current: bool,
    /// Seconds since the epoch: the commit time for branches and lightweight
    /// tags, the tagger time for annotated tags.
    pub time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitItem {
    pub sha: String,
    pub short_sha: String,
    /// The first line of the message.
    pub subject: String,
    pub author_name: String,
    /// The commit time, in seconds since the epoch.
    pub time: i64,
}

impl Repo {
    /// Local branches, then remote-tracking branches, each sorted by most
    /// recent commit. The current branch comes first.
    pub fn list_branches(&self) -> Result<Vec<RefItem>> {
        let repo = self.local();
        let head = repo.head_name()?;
        let platform = repo.references()?;
        let mut items = Vec::new();
        for (kind, iter) in [
            (RefKind::LocalBranch, platform.local_branches()?),
            (RefKind::RemoteBranch, platform.remote_branches()?),
        ] {
            for reference in iter {
                let mut reference = reference?;
                // Skip symbolic refs such as `origin/HEAD`.
                if reference.target().try_id().is_none() {
                    continue;
                }
                let Ok(commit) = reference.peel_to_commit() else {
                    continue;
                };
                let name = reference.name();
                items.push(RefItem {
                    name: name.shorten().to_string(),
                    full_name: name.as_bstr().to_string(),
                    kind,
                    commit: commit.id.to_string(),
                    current: head.as_ref().is_some_and(|head| head.as_ref() == name),
                    time: commit.time()?.seconds,
                });
            }
        }
        items.sort_by(|a, b| {
            b.current
                .cmp(&a.current)
                .then((a.kind == RefKind::RemoteBranch).cmp(&(b.kind == RefKind::RemoteBranch)))
                .then(b.time.cmp(&a.time))
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(items)
    }

    /// Tags that point to commits, newest first.
    pub fn list_tags(&self) -> Result<Vec<RefItem>> {
        let repo = self.local();
        let mut items = Vec::new();
        for reference in repo.references()?.tags()? {
            let mut reference = reference?;
            let Some(id) = reference.target().try_id().map(ToOwned::to_owned) else {
                continue;
            };
            let tagger_time = match repo.find_object(id)?.try_into_tag() {
                Ok(tag) => tag.tagger()?.map(|tagger| tagger.time()).transpose()?,
                Err(_) => None,
            };
            let Ok(commit) = reference.peel_to_commit() else {
                continue;
            };
            let time = match tagger_time {
                Some(time) => time.seconds,
                None => commit.time()?.seconds,
            };
            let name = reference.name();
            debug_assert_eq!(name.category(), Some(Category::Tag));
            items.push(RefItem {
                name: name.shorten().to_string(),
                full_name: name.as_bstr().to_string(),
                kind: RefKind::Tag,
                commit: commit.id.to_string(),
                current: false,
                time,
            });
        }
        items.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| a.name.cmp(&b.name)));
        Ok(items)
    }

    /// Commits reachable from `HEAD`, newest first, skipping the first `offset`.
    pub fn list_commits(&self, offset: usize, limit: usize) -> Result<Vec<CommitItem>> {
        let repo = self.local();
        let Some(walk) = head_walk(&repo)? else {
            return Ok(Vec::new());
        };
        walk.skip(offset)
            .take(limit)
            .map(|info| commit_item(&repo.find_commit(info?.id)?))
            .collect()
    }

    /// Commits reachable from `HEAD` whose message contains `query`, ignoring
    /// case, newest first. Looks at no more than [`SEARCH_COMMITS_SCAN_LIMIT`] commits.
    pub fn search_commits(&self, query: &str, limit: usize) -> Result<Vec<CommitItem>> {
        let repo = self.local();
        let Some(walk) = head_walk(&repo)? else {
            return Ok(Vec::new());
        };
        let query = query.to_lowercase();
        let mut found = Vec::new();
        for info in walk.take(SEARCH_COMMITS_SCAN_LIMIT) {
            if found.len() == limit {
                break;
            }
            let commit = repo.find_commit(info?.id)?;
            let message = commit.message_raw_sloppy().to_str_lossy().to_lowercase();
            if message.contains(&query) {
                found.push(commit_item(&commit)?);
            }
        }
        Ok(found)
    }

    /// The commits whose SHA starts with `prefix`, if it's 4 to 40 hex
    /// characters. An ambiguous prefix returns every match, newest first.
    pub fn resolve_commit_prefix(&self, prefix: &str) -> Result<Vec<CommitItem>> {
        if !(4..=40).contains(&prefix.len()) || !prefix.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(Vec::new());
        }
        let repo = self.local();
        let prefix =
            gix::hash::Prefix::from_hex(&prefix.to_ascii_lowercase()).map_err(Error::internal)?;
        let mut candidates = HashSet::new();
        repo.objects
            .lookup_prefix(prefix, Some(&mut candidates))
            .map_err(Error::internal)?;
        let mut items = Vec::new();
        for id in candidates {
            if let Ok(commit) = repo.find_commit(id) {
                items.push(commit_item(&commit)?);
            }
        }
        items.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| a.sha.cmp(&b.sha)));
        Ok(items)
    }
}

/// A walk from `HEAD` by commit time, or `None` when there are no commits yet.
fn head_walk(repo: &gix::Repository) -> Result<Option<gix::revision::Walk<'_>>> {
    let Some(head) = repo.head()?.id() else {
        return Ok(None);
    };
    let walk = repo
        .rev_walk([head])
        .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
        .all()?;
    Ok(Some(walk))
}

fn commit_item(commit: &gix::Commit<'_>) -> Result<CommitItem> {
    let message = commit.message()?;
    let author = commit.author()?;
    Ok(CommitItem {
        sha: commit.id.to_string(),
        short_sha: commit.id.to_hex_with_len(7).to_string(),
        subject: message.summary().to_str_lossy().into_owned(),
        author_name: author.name.to_str_lossy().into_owned(),
        time: commit.time()?.seconds,
    })
}
