mod common;

use common::Fixture;
use turbodiff_repo::{Error, HeadInfo, Repo};

#[test]
fn opens_the_enclosing_repository_from_a_subfolder() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(fixture.path.join("src")).unwrap();
    let info = repo.info().unwrap();
    assert_eq!(info.root, fixture.path.canonicalize().unwrap());
    assert_eq!(info.name, "worktree");
    assert_eq!(
        info.head,
        HeadInfo {
            branch: Some("main".into()),
            commit: Some(fixture.git(&["rev-parse", "HEAD"]).trim().into()),
        }
    );
}

#[test]
fn a_repository_without_commits_has_no_head_commit() {
    let fixture = Fixture::build("empty");
    let info = Repo::open(&fixture.path).unwrap().info().unwrap();
    assert_eq!(info.head.branch.as_deref(), Some("main"));
    assert_eq!(info.head.commit, None);
}

#[test]
fn a_detached_head_has_no_branch() {
    let fixture = Fixture::build("refs");
    fixture.git(&["switch", "-q", "--detach", "v0.1"]);
    let info = Repo::open(&fixture.path).unwrap().info().unwrap();
    assert_eq!(info.head.branch, None);
    assert_eq!(
        info.head.commit.as_deref(),
        Some(fixture.git(&["rev-parse", "v0.1"]).trim())
    );
}

#[test]
fn a_folder_outside_any_repository_is_not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    let err = Repo::open(dir.path()).err().unwrap();
    assert!(matches!(err, Error::NotARepository(path) if path == dir.path()));
}

#[test]
fn a_missing_folder_is_not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    let err = Repo::open(dir.path().join("missing")).err().unwrap();
    assert!(matches!(err, Error::NotARepository(_)));
}

#[test]
fn a_bare_repository_is_not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    common::git(dir.path(), &["init", "-q", "--bare", "bare.git"]);
    let err = Repo::open(dir.path().join("bare.git")).err().unwrap();
    assert!(matches!(err, Error::NotARepository(_)));
}
