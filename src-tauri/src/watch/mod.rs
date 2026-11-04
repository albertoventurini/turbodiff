//! Watches a repository for changes that can change its diff (TECH.md §4.6).
//!
//! Plain Rust with no Tauri types, so it can be tested on its own. The app
//! decides what a change means for a window's comparison.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use notify::EventKind;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use turbodiff_repo::Repo;

use crate::app::lock;

/// How long the repository has to be quiet before a change is reported (PRD
/// §6.9).
pub const DEBOUNCE: Duration = Duration::from_millis(300);
/// The longest a change waits while the repository never goes quiet, such as
/// during a long build that writes tracked files.
pub const MAX_DELAY: Duration = Duration::from_secs(2);
/// How often an idle watcher checks whether it was stopped.
const IDLE_POLL: Duration = Duration::from_millis(250);

/// What changed in one debounced burst.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Trigger {
    /// `HEAD`, the index or a ref.
    pub git: bool,
    /// A work-tree file that git doesn't ignore.
    pub work_tree: bool,
}

impl Trigger {
    fn any(self) -> bool {
        self.git || self.work_tree
    }

    fn merge(&mut self, other: Trigger) {
        self.git |= other.git;
        self.work_tree |= other.work_tree;
    }
}

/// Where a repository's files are.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
}

impl Layout {
    pub fn of(repo: &Repo) -> Self {
        Self {
            root: repo.root().to_owned(),
            git_dir: repo.git_dir().to_owned(),
            common_dir: repo.common_dir().to_owned(),
        }
    }
}

/// Whether `path`, in the git dir or the common dir, is something a diff
/// depends on: `HEAD`, the index, `packed-refs` or a ref. Lock files,
/// objects, logs and the like are not.
fn is_git_state(layout: &Layout, path: &Path) -> bool {
    if path.extension().is_some_and(|ext| ext == "lock") {
        return false;
    }
    let in_git_dir = path.strip_prefix(&layout.git_dir).ok();
    let in_common_dir = path.strip_prefix(&layout.common_dir).ok();
    in_git_dir.is_some_and(|path| path == Path::new("HEAD") || path == Path::new("index"))
        || in_common_dir
            .is_some_and(|path| path == Path::new("packed-refs") || path.starts_with("refs"))
}

fn in_git_dirs(layout: &Layout, path: &Path) -> bool {
    path.starts_with(&layout.git_dir)
        || path.starts_with(&layout.common_dir)
        || path.starts_with(layout.root.join(".git"))
}

/// Sorts the `(path, is_dir)` of changed paths into a [`Trigger`].
/// `ignored` answers, for work-tree paths, which ones git ignores
/// ([`Repo::ignored`]).
pub fn classify(
    layout: &Layout,
    paths: &[(PathBuf, bool)],
    ignored: impl FnOnce(&[(&Path, bool)]) -> Vec<bool>,
) -> Trigger {
    let mut trigger = Trigger::default();
    let mut work_tree = Vec::new();
    for (path, is_dir) in paths {
        if in_git_dirs(layout, path) {
            trigger.git |= is_git_state(layout, path);
        } else if path.starts_with(&layout.root) {
            work_tree.push((path.as_path(), *is_dir));
        }
    }
    if !work_tree.is_empty() {
        trigger.work_tree = ignored(&work_tree).iter().any(|ignored| !ignored);
    }
    trigger
}

/// Watches one repository until dropped.
pub struct Watcher {
    _watcher: Arc<Mutex<RecommendedWatcher>>,
    stop: Arc<AtomicBool>,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Watcher {
    /// Starts watching `repo`, calling `on_change` on the watcher's thread
    /// after each debounced burst of changes that matter.
    pub fn start(
        repo: Arc<Repo>,
        on_change: impl Fn(Trigger) + Send + 'static,
    ) -> notify::Result<Self> {
        let (sender, events) = mpsc::channel();
        let watcher = Arc::new(Mutex::new(notify::recommended_watcher(sender)?));
        let layout = Layout::of(&repo);
        watch_git_dirs(&mut lock(&watcher), &layout);
        watch_work_tree(&mut lock(&watcher), &repo, repo.root());
        let stop = Arc::new(AtomicBool::new(false));
        let worker = Worker {
            repo,
            layout,
            watcher: Arc::downgrade(&watcher),
            stop: stop.clone(),
        };
        std::thread::Builder::new()
            .name("turbodiff watcher".into())
            .spawn(move || worker.run(events, on_change))?;
        Ok(Self {
            _watcher: watcher,
            stop,
        })
    }
}

fn watch(watcher: &mut RecommendedWatcher, path: &Path, mode: RecursiveMode) {
    if let Err(err) = watcher.watch(path, mode)
        && path.exists()
    {
        eprintln!("turbodiff: can't watch {}: {err}", path.display());
    }
}

/// `HEAD` and the index in the git dir, and `packed-refs` and `refs/` in the
/// common dir. On macOS the work tree's recursive watch already covers a
/// `.git` folder inside it.
fn watch_git_dirs(watcher: &mut RecommendedWatcher, layout: &Layout) {
    let covered = |path: &Path| cfg!(target_os = "macos") && path.starts_with(&layout.root);
    if !covered(&layout.git_dir) {
        watch(watcher, &layout.git_dir, RecursiveMode::NonRecursive);
    }
    if !covered(&layout.common_dir) {
        if layout.common_dir != layout.git_dir {
            watch(watcher, &layout.common_dir, RecursiveMode::NonRecursive);
        }
        watch(
            watcher,
            &layout.common_dir.join("refs"),
            RecursiveMode::Recursive,
        );
    }
}

/// On Linux, inotify needs a watch for every folder, so only folders git
/// doesn't ignore are watched, one by one. Elsewhere one recursive watch on
/// the root is cheap, and events are filtered instead.
fn watch_work_tree(watcher: &mut RecommendedWatcher, repo: &Repo, dir: &Path) {
    if cfg!(target_os = "linux") {
        match repo.unignored_dirs(dir) {
            Ok(dirs) => {
                for dir in dirs {
                    watch(watcher, &dir, RecursiveMode::NonRecursive);
                }
            }
            Err(err) => eprintln!("turbodiff: can't list folders to watch: {err}"),
        }
    } else if dir == repo.root() {
        watch(watcher, dir, RecursiveMode::Recursive);
    }
}

struct Worker {
    repo: Arc<Repo>,
    layout: Layout,
    watcher: Weak<Mutex<RecommendedWatcher>>,
    stop: Arc<AtomicBool>,
}

impl Worker {
    /// Collects changes until the repository has been quiet for `DEBOUNCE`
    /// (or `MAX_DELAY` has passed), then reports them.
    fn run(self, events: Receiver<notify::Result<notify::Event>>, on_change: impl Fn(Trigger)) {
        let mut pending = Trigger::default();
        let mut first = Instant::now();
        let mut last = first;
        loop {
            if self.stop.load(Ordering::Acquire) {
                return;
            }
            let wait = if pending.any() {
                (last + DEBOUNCE)
                    .min(first + MAX_DELAY)
                    .saturating_duration_since(Instant::now())
            } else {
                IDLE_POLL
            };
            let batch = match events.recv_timeout(wait) {
                Ok(event) => {
                    let mut batch = vec![event];
                    batch.extend(events.try_iter());
                    batch
                }
                Err(RecvTimeoutError::Timeout) => {
                    if pending.any() {
                        on_change(std::mem::take(&mut pending));
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            };
            let trigger = self.handle(batch);
            if trigger.any() {
                last = Instant::now();
                if !pending.any() {
                    first = last;
                }
                pending.merge(trigger);
            }
        }
    }

    fn handle(&self, batch: Vec<notify::Result<notify::Event>>) -> Trigger {
        let mut paths = Vec::new();
        for event in batch {
            let event = match event {
                Ok(event) => event,
                Err(err) => {
                    eprintln!("turbodiff: watch error: {err}");
                    continue;
                }
            };
            // Reading files, ours included, isn't a change.
            if matches!(event.kind, EventKind::Access(_)) {
                continue;
            }
            for path in event.paths {
                let is_dir = path.is_dir();
                if is_dir && event.kind.is_create() {
                    self.watch_new_dir(&path);
                }
                paths.push((path, is_dir));
            }
        }
        if paths.is_empty() {
            return Trigger::default();
        }
        classify(&self.layout, &paths, |paths| {
            self.repo.ignored(paths).unwrap_or_else(|err| {
                eprintln!("turbodiff: can't check ignored files: {err}");
                vec![false; paths.len()]
            })
        })
    }

    fn watch_new_dir(&self, dir: &Path) {
        if cfg!(target_os = "linux")
            && !in_git_dirs(&self.layout, dir)
            && let Some(watcher) = self.watcher.upgrade()
        {
            watch_work_tree(&mut lock(&watcher), &self.repo, dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;

    fn layout() -> Layout {
        Layout {
            root: "/repo".into(),
            git_dir: "/repo/.git".into(),
            common_dir: "/repo/.git".into(),
        }
    }

    fn trigger(layout: &Layout, paths: &[&str]) -> Trigger {
        let paths: Vec<_> = paths
            .iter()
            .map(|path| (PathBuf::from(path), false))
            .collect();
        classify(layout, &paths, |paths| {
            paths
                .iter()
                .map(|(path, _)| path.starts_with("/repo/build"))
                .collect()
        })
    }

    #[test]
    fn head_the_index_and_refs_are_git_changes() {
        let layout = layout();
        for path in [
            "/repo/.git/HEAD",
            "/repo/.git/index",
            "/repo/.git/packed-refs",
            "/repo/.git/refs/heads/main",
            "/repo/.git/refs/remotes/origin/feature/x",
        ] {
            assert_eq!(
                trigger(&layout, &[path]),
                Trigger {
                    git: true,
                    work_tree: false
                },
                "{path}"
            );
        }
    }

    #[test]
    fn other_git_internals_are_ignored() {
        let layout = layout();
        for path in [
            "/repo/.git/index.lock",
            "/repo/.git/refs/heads/main.lock",
            "/repo/.git/objects/ab/cdef",
            "/repo/.git/logs/HEAD",
            "/repo/.git/ORIG_HEAD",
            "/repo/.git/FETCH_HEAD",
            "/repo/.git/config",
        ] {
            assert_eq!(trigger(&layout, &[path]), Trigger::default(), "{path}");
        }
    }

    #[test]
    fn ignored_work_tree_files_and_paths_elsewhere_are_ignored() {
        let layout = layout();
        assert_eq!(trigger(&layout, &["/repo/build/out.o"]), Trigger::default());
        assert_eq!(trigger(&layout, &["/elsewhere/file"]), Trigger::default());
        assert_eq!(
            trigger(&layout, &["/repo/build/out.o", "/repo/src/main.rs"]),
            Trigger {
                git: false,
                work_tree: true
            }
        );
    }

    #[test]
    fn a_linked_worktree_finds_refs_in_the_common_dir() {
        let layout = Layout {
            root: "/linked".into(),
            git_dir: "/repo/.git/worktrees/linked".into(),
            common_dir: "/repo/.git".into(),
        };
        let git = Trigger {
            git: true,
            work_tree: false,
        };
        assert_eq!(trigger(&layout, &["/repo/.git/worktrees/linked/HEAD"]), git);
        assert_eq!(trigger(&layout, &["/repo/.git/refs/heads/other"]), git);
        // The main work tree's HEAD and index aren't this worktree's.
        assert_eq!(trigger(&layout, &["/repo/.git/HEAD"]), Trigger::default());
        assert_eq!(trigger(&layout, &["/repo/.git/index"]), Trigger::default());
        // Its `.git` is a file pointing to the git dir.
        assert_eq!(trigger(&layout, &["/linked/.git"]), Trigger::default());
    }

    /// Waits for the next trigger, or `None` after `timeout`.
    fn next(triggers: &Receiver<Trigger>, timeout: Duration) -> Option<Trigger> {
        triggers.recv_timeout(timeout).ok()
    }

    #[test]
    fn reports_edits_once_per_burst_and_skips_ignored_files() {
        let fixture = Fixture::build("worktree");
        let repo = Arc::new(Repo::open(&fixture.path).unwrap());
        let (sender, triggers) = mpsc::channel();
        let _watcher = Watcher::start(repo, move |trigger| {
            let _ = sender.send(trigger);
        })
        .unwrap();
        let quiet = DEBOUNCE * 3;

        std::fs::write(fixture.path.join("build/out.txt"), "rebuilt\n").unwrap();
        std::fs::write(fixture.path.join("debug.log"), "more\n").unwrap();
        assert_eq!(next(&triggers, quiet), None);

        for i in 0..5 {
            std::fs::write(fixture.path.join("modified.txt"), format!("edit {i}\n")).unwrap();
            std::fs::write(fixture.path.join("src/lib.rs"), format!("edit {i}\n")).unwrap();
        }
        assert_eq!(
            next(&triggers, Duration::from_secs(5)),
            Some(Trigger {
                git: false,
                work_tree: true
            })
        );
        assert_eq!(next(&triggers, quiet), None);

        // A folder created after watching started is watched too.
        std::fs::create_dir(fixture.path.join("later")).unwrap();
        assert!(next(&triggers, Duration::from_secs(5)).is_some());
        std::fs::write(fixture.path.join("later/file.txt"), "new\n").unwrap();
        assert!(next(&triggers, Duration::from_secs(5)).is_some());

        fixture.git(&["add", "modified.txt"]);
        assert_eq!(
            next(&triggers, Duration::from_secs(5)),
            Some(Trigger {
                git: true,
                work_tree: false
            })
        );
    }

    #[test]
    fn stops_when_dropped() {
        let fixture = Fixture::build("worktree");
        let repo = Arc::new(Repo::open(&fixture.path).unwrap());
        let (sender, triggers) = mpsc::channel();
        let watcher = Watcher::start(repo, move |trigger| {
            let _ = sender.send(trigger);
        })
        .unwrap();
        drop(watcher);
        std::fs::write(fixture.path.join("modified.txt"), "edit\n").unwrap();
        // The worker thread exits, dropping the sender.
        assert!(matches!(
            triggers.recv_timeout(Duration::from_secs(5)),
            Err(RecvTimeoutError::Disconnected)
        ));
    }
}
