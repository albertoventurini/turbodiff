//! A window's repository session: one repository, one comparison, the cached
//! list of changed files, and the `generation` they were computed for
//! (TECH.md §2, §5).

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use turbodiff_repo::{
    Algorithm, Cancel, ChangedFile, Changes, Comparison, FileEntry, Repo, Totals,
};

use crate::error::{AppError, AppResult};

/// The result of `set_comparison`.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    pub generation: u32,
    pub files: Vec<FileEntry>,
    pub totals: Totals,
    /// Three-dot was asked for, but the sides have no merge base, so this is
    /// a two-dot comparison (TECH.md §4.3).
    pub no_merge_base: bool,
}

/// A response that depends on the repository, with the generation it was
/// computed for. The frontend discards responses older than the latest one.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Versioned<T> {
    pub generation: u32,
    pub value: T,
}

pub struct Session {
    repo: Arc<Repo>,
    state: Mutex<State>,
}

struct State {
    comparison: Comparison,
    /// Increased each time the comparison is set (and, later, refreshed).
    generation: u32,
    /// Cancels the listing for the current generation.
    cancel: Cancel,
    /// The changed files for `generation`, once listed.
    changes: Option<Arc<Changes>>,
}

/// What a listing needs to run without holding the session's lock.
pub struct Ticket {
    pub generation: u32,
    pub comparison: Comparison,
    cancel: Cancel,
}

impl Session {
    pub fn new(repo: Repo, comparison: Comparison) -> Self {
        Self {
            repo: Arc::new(repo),
            state: Mutex::new(State {
                comparison,
                generation: 0,
                cancel: Cancel::new(),
                changes: None,
            }),
        }
    }

    pub fn repo(&self) -> &Arc<Repo> {
        &self.repo
    }

    pub fn root(&self) -> &Path {
        self.repo.root()
    }

    pub fn comparison(&self) -> Comparison {
        self.state().comparison.clone()
    }

    pub fn generation(&self) -> u32 {
        self.state().generation
    }

    /// Switches to `comparison`: starts a new generation, drops the cached
    /// changes and cancels the listing of the previous generation.
    pub fn begin(&self, comparison: Comparison) -> Ticket {
        let mut state = self.state();
        state.cancel.cancel();
        state.cancel = Cancel::new();
        state.generation += 1;
        state.comparison = comparison.clone();
        state.changes = None;
        Ticket {
            generation: state.generation,
            comparison,
            cancel: state.cancel.clone(),
        }
    }

    /// Lists the changes for `ticket`. Blocks, so run it on the blocking
    /// thread pool. `Cancelled` if a newer generation started meanwhile.
    pub fn list_changes(&self, ticket: &Ticket, algorithm: Algorithm) -> AppResult<ChangeSet> {
        let changes =
            self.repo
                .changes_cancellable(&ticket.comparison, algorithm, &ticket.cancel)?;
        self.finish(ticket, changes)
    }

    fn finish(&self, ticket: &Ticket, changes: Changes) -> AppResult<ChangeSet> {
        let mut state = self.state();
        if state.generation != ticket.generation {
            return Err(AppError::Cancelled);
        }
        let set = ChangeSet {
            generation: ticket.generation,
            files: changes.entries().cloned().collect(),
            totals: changes.totals,
            no_merge_base: changes.no_merge_base,
        };
        state.changes = Some(Arc::new(changes));
        Ok(set)
    }

    /// The changed file at `path` in the current list, with the generation
    /// and whether whitespace is hidden.
    pub fn file(&self, path: &str) -> AppResult<(u32, ChangedFile, bool)> {
        let state = self.state();
        let file = state
            .changes
            .as_ref()
            .and_then(|changes| {
                // `Changes::files` is sorted by path.
                let index = changes
                    .files
                    .binary_search_by(|file| file.entry.path.as_str().cmp(path))
                    .ok()?;
                Some(changes.files[index].clone())
            })
            .ok_or_else(|| AppError::FileNotFound { path: path.into() })?;
        Ok((state.generation, file, state.comparison.ignore_whitespace))
    }

    /// Stops any listing in progress, when the window closes.
    pub fn close(&self) {
        self.state().cancel.cancel();
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use turbodiff_repo::{CompareSide, Mode};

    use super::*;
    use crate::test_support::Fixture;

    fn session(fixture: &Fixture) -> Session {
        Session::new(Repo::open(&fixture.path).unwrap(), Comparison::default())
    }

    #[test]
    fn each_comparison_starts_a_new_generation() {
        let fixture = Fixture::build("worktree");
        let session = session(&fixture);
        assert_eq!(session.generation(), 0);
        let ticket = session.begin(Comparison::default());
        let set = session.list_changes(&ticket, Algorithm::Histogram).unwrap();
        assert_eq!(set.generation, 1);
        assert!(!set.files.is_empty());
        assert_eq!(set.totals.files as usize, set.files.len());

        let ticket = session.begin(Comparison::default());
        assert_eq!(ticket.generation, 2);
        assert_eq!(session.generation(), 2);
    }

    #[test]
    fn a_new_comparison_cancels_the_previous_listing() {
        let fixture = Fixture::build("three_dot");
        let session = session(&fixture);
        let first = session.begin(Comparison::default());
        let second = session.begin(Comparison {
            base: "main".into(),
            compare: CompareSide::Commit {
                spec: "feature".into(),
            },
            mode: Mode::TwoDot,
            ignore_whitespace: false,
        });
        assert!(first.cancel.is_cancelled());
        assert!(!second.cancel.is_cancelled());
        assert_eq!(
            session
                .list_changes(&first, Algorithm::Histogram)
                .unwrap_err(),
            AppError::Cancelled
        );
        let set = session.list_changes(&second, Algorithm::Histogram).unwrap();
        assert_eq!(set.generation, 2);
        assert_eq!(session.comparison(), second.comparison);
    }

    #[test]
    fn a_listing_that_finishes_late_is_discarded() {
        let fixture = Fixture::build("worktree");
        let session = session(&fixture);
        let first = session.begin(Comparison::default());
        // Computed before the comparison changed, finished after.
        let changes = session
            .repo()
            .changes(&first.comparison, Algorithm::Histogram)
            .unwrap();
        let second = session.begin(Comparison::default());
        assert_eq!(
            session.finish(&first, changes).unwrap_err(),
            AppError::Cancelled
        );
        // The late result didn't replace anything.
        assert!(matches!(
            session.file("anything"),
            Err(AppError::FileNotFound { .. })
        ));
        session.list_changes(&second, Algorithm::Histogram).unwrap();
    }

    #[test]
    fn files_are_looked_up_in_the_current_list() {
        let fixture = Fixture::build("worktree");
        let session = session(&fixture);
        assert!(matches!(
            session.file("a.txt"),
            Err(AppError::FileNotFound { .. })
        ));
        let ticket = session.begin(Comparison::default());
        let set = session.list_changes(&ticket, Algorithm::Histogram).unwrap();
        for entry in &set.files {
            let (generation, file, ignore_whitespace) = session.file(&entry.path).unwrap();
            assert_eq!(generation, 1);
            assert_eq!(&file.entry, entry);
            assert!(!ignore_whitespace);
        }
        assert!(matches!(
            session.file("no/such/file"),
            Err(AppError::FileNotFound { .. })
        ));
    }
}
