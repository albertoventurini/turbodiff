//! Nothing TurboDiff does changes a repository (PRD §8): not `.git`, not the
//! work tree, not even file timestamps.

mod common;

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

use common::Fixture;
use turbodiff_repo::{Algorithm, Comparison, Repo};

/// Every file under `dir` with its modification time and a hash of its content.
fn snapshot(dir: &Path) -> BTreeMap<String, (SystemTime, u64)> {
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.path().symlink_metadata().unwrap();
            if metadata.is_dir() {
                stack.push(entry.path());
                continue;
            }
            let mut hasher = DefaultHasher::new();
            if metadata.is_file() {
                std::fs::read(entry.path()).unwrap().hash(&mut hasher);
            }
            files.insert(
                entry.path().display().to_string(),
                (metadata.modified().unwrap(), hasher.finish()),
            );
        }
    }
    files
}

#[test]
fn reading_a_repository_changes_nothing_on_disk() {
    for name in [
        "worktree",
        "empty",
        "renames",
        "binary",
        "line_endings",
        "three_dot",
    ] {
        let fixture = Fixture::build(name);
        let before = snapshot(fixture.path.parent().unwrap());

        let repo = Repo::open(&fixture.path).unwrap();
        repo.info().unwrap();
        repo.list_branches().unwrap();
        repo.list_tags().unwrap();
        repo.list_commits(0, 50).unwrap();
        repo.search_commits("a", 50).unwrap();
        repo.resolve_commit_prefix("abcd").unwrap();
        for ignore_whitespace in [false, true] {
            let comparison = Comparison {
                ignore_whitespace,
                ..Comparison::default()
            };
            repo.changes(&comparison, Algorithm::Histogram).unwrap();
        }

        assert_eq!(before, snapshot(fixture.path.parent().unwrap()), "{name}");
    }
}
