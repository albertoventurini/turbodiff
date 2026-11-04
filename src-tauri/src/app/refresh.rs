//! Live refresh: watching a window's repository and refreshing its file list
//! when it changes (PRD §6.9, TECH.md §4.6).

use std::sync::{Arc, Weak};

use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use turbodiff_repo::{CompareSide, Comparison};

use super::AppState;
use crate::error::AppError;
use crate::events::{ComparisonInvalid, RepoChanged};
use crate::session::Session;
use crate::watch::{Trigger, Watcher};

/// Whether a change refreshes `comparison`. Work-tree edits only matter when
/// the work tree is one of the sides; a moved ref or `HEAD` always may.
pub fn should_refresh(comparison: &Comparison, trigger: Trigger) -> bool {
    trigger.git || (trigger.work_tree && matches!(comparison.compare, CompareSide::WorkTree))
}

/// Starts watching `session`'s repository for window `label`, off the async
/// runtime, since listing the folders to watch can take a while. The watcher
/// lives in the window's slot, and stops when the slot is replaced or the
/// window closes.
pub fn start(app: &AppHandle, label: String, session: Arc<Session>) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let watcher = {
            let app = app.clone();
            let label = label.clone();
            let weak = Arc::downgrade(&session);
            Watcher::start(session.repo().clone(), move |trigger| {
                on_change(&app, &label, &weak, trigger)
            })
        };
        let watcher = match watcher {
            Ok(watcher) => watcher,
            Err(err) => {
                eprintln!("turbodiff: can't watch {}: {err}", session.root().display());
                return;
            }
        };
        let state = app.state::<AppState>();
        let mut windows = state.windows();
        if let Some(slot) = windows.get_mut(&label)
            && slot
                .session
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &session))
        {
            slot.watcher = Some(watcher);
        }
    });
}

/// Rebuilds the file list on the blocking pool and sends the result. A newer
/// change, or a new comparison, cancels a refresh still running.
fn on_change(app: &AppHandle, label: &str, session: &Weak<Session>, trigger: Trigger) {
    let Some(session) = session.upgrade() else {
        return;
    };
    if !should_refresh(&session.comparison(), trigger) {
        return;
    }
    let algorithm = app.state::<AppState>().settings().diff_algorithm;
    let ticket = session.begin_refresh();
    let app = app.clone();
    let label = label.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let result = session
            .refresh(&ticket, algorithm)
            .and_then(|(changes, files_changed)| {
                let head = session.repo().info()?.head;
                Ok(RepoChanged {
                    comparison: ticket.comparison.clone(),
                    changes,
                    files_changed,
                    head,
                })
            });
        let sent = match result {
            Ok(event) => event.emit_to(&app, &label),
            Err(AppError::RefNotFound { name }) => ComparisonInvalid { name }.emit_to(&app, &label),
            Err(AppError::Cancelled) => Ok(()),
            // Such as a file read halfway through a write. The next change
            // tries again.
            Err(err) => {
                eprintln!(
                    "turbodiff: can't refresh {}: {err}",
                    session.root().display()
                );
                Ok(())
            }
        };
        if let Err(err) = sent {
            eprintln!("turbodiff: can't send repo-changed: {err}");
        }
    });
}

#[cfg(test)]
mod tests {
    use turbodiff_repo::Mode;

    use super::*;

    const GIT: Trigger = Trigger {
        git: true,
        work_tree: false,
    };
    const WORK_TREE: Trigger = Trigger {
        git: false,
        work_tree: true,
    };

    #[test]
    fn a_work_tree_comparison_refreshes_on_any_change() {
        let comparison = Comparison::default();
        assert!(should_refresh(&comparison, GIT));
        assert!(should_refresh(&comparison, WORK_TREE));
    }

    #[test]
    fn a_commit_comparison_refreshes_only_when_refs_move() {
        let comparison = Comparison {
            base: "main".into(),
            compare: CompareSide::Commit {
                spec: "feature".into(),
            },
            mode: Mode::ThreeDot,
            ignore_whitespace: false,
        };
        assert!(should_refresh(&comparison, GIT));
        assert!(!should_refresh(&comparison, WORK_TREE));
    }
}
