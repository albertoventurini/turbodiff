mod common;

use std::path::Path;

use common::Fixture;
use turbodiff_repo::Repo;

fn ignored(repo: &Repo, paths: &[(&str, bool)]) -> Vec<bool> {
    let paths: Vec<_> = paths
        .iter()
        .map(|(path, is_dir)| (repo.root().join(path), *is_dir))
        .collect();
    let paths: Vec<_> = paths
        .iter()
        .map(|(path, is_dir)| (path.as_path(), *is_dir))
        .collect();
    repo.ignored(&paths).unwrap()
}

#[test]
fn gitignore_and_info_exclude_paths_are_ignored() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    assert_eq!(
        ignored(
            &repo,
            &[
                ("build", true),
                ("build/out.txt", false),
                ("build/deep/er/file.txt", false),
                ("debug.log", false),
                ("secret.txt", false),
                ("modified.txt", false),
                ("untracked.txt", false),
                ("src", true),
                ("newdir/sub/file.txt", false),
            ]
        ),
        [true, true, true, true, true, false, false, false, false]
    );
}

#[test]
fn nested_gitignore_files_and_edits_to_them_apply() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    assert_eq!(ignored(&repo, &[("src/gen.rs", false)]), [false]);
    std::fs::write(fixture.path.join("src/.gitignore"), "gen.rs\n").unwrap();
    assert_eq!(ignored(&repo, &[("src/gen.rs", false)]), [true]);
}

#[test]
fn tracked_files_are_never_ignored() {
    let fixture = Fixture::build("worktree");
    fixture.git(&["add", "-f", "build/out.txt"]);
    let repo = Repo::open(&fixture.path).unwrap();
    assert_eq!(
        ignored(
            &repo,
            &[
                ("build/out.txt", false),
                ("build", true),
                ("debug.log", false)
            ]
        ),
        [false, false, true]
    );
}

#[test]
fn paths_outside_the_work_tree_are_not_ignored() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    assert_eq!(
        repo.ignored(&[(Path::new("/elsewhere/debug.log"), false)])
            .unwrap(),
        [false]
    );
}

#[test]
fn unignored_dirs_leave_out_git_and_ignored_folders() {
    let fixture = Fixture::build("worktree");
    std::fs::create_dir_all(fixture.path.join("build/nested")).unwrap();
    let repo = Repo::open(&fixture.path).unwrap();
    let root = repo.root();
    let dirs: Vec<_> = repo
        .unignored_dirs(root)
        .unwrap()
        .iter()
        .map(|dir| {
            dir.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(dirs, ["", "newdir", "newdir/sub", "src"]);
    assert_eq!(
        repo.unignored_dirs(&root.join("newdir")).unwrap(),
        [root.join("newdir"), root.join("newdir/sub")]
    );
}

#[test]
fn a_linked_worktree_has_its_own_git_dir_and_shares_the_common_dir() {
    let fixture = Fixture::build("worktree");
    fixture.git(&["worktree", "add", "-q", "-b", "other", "../linked"]);
    let main = Repo::open(&fixture.path).unwrap();
    let linked = Repo::open(fixture.path.join("../linked")).unwrap();
    assert_eq!(main.git_dir(), main.root().join(".git"));
    assert_eq!(main.common_dir(), main.git_dir());
    assert_eq!(linked.common_dir(), main.git_dir());
    assert_eq!(linked.git_dir(), main.git_dir().join("worktrees/linked"));
}
