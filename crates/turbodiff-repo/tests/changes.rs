//! Changed files compared with `git diff --name-status -M` and `git diff
//! --numstat --diff-algorithm=histogram` for every fixture (TECH.md §10.1).
//! Hunk boundaries are not compared.

mod common;

use std::path::Path;

use common::Fixture;
use turbodiff_repo::{Algorithm, Changes, CompareSide, Comparison, Mode, Repo, Status};

/// Known differences from git, as `(fixture, path, reason)`. git's counts for
/// these paths are replaced with binary (`-`) before comparing.
const ALLOWED_DIFFERENCES: &[(&str, &str, &str)] = &[(
    "binary",
    "latin1.txt",
    "not valid UTF-8, so TurboDiff shows it as binary (PRD §8); git counts its lines",
)];

fn worktree(base: &str, mode: Mode) -> Comparison {
    Comparison {
        base: base.into(),
        compare: CompareSide::WorkTree,
        mode,
        ignore_whitespace: false,
    }
}

fn commits(base: &str, compare: &str, mode: Mode) -> Comparison {
    Comparison {
        base: base.into(),
        compare: CompareSide::Commit {
            spec: compare.into(),
        },
        mode,
        ignore_whitespace: false,
    }
}

/// `git diff --name-status -z` output as lines like `R086\told\tnew` or `M\tpath`.
fn parse_name_status(output: &str) -> Vec<String> {
    let mut fields = output.split('\0').filter(|f| !f.is_empty());
    let mut lines = Vec::new();
    while let Some(status) = fields.next() {
        let line = if status.starts_with('R') || status.starts_with('C') {
            let old = fields.next().unwrap();
            let new = fields.next().unwrap();
            format!("{status}\t{old}\t{new}")
        } else {
            format!("{status}\t{}", fields.next().unwrap())
        };
        lines.push(line);
    }
    lines.sort();
    lines
}

/// `git diff --numstat -z` output as lines like `3\t1\tpath`, `-\t-\tpath` or
/// `0\t0\told\tnew` for renames.
fn parse_numstat(output: &str) -> Vec<String> {
    let mut fields = output.split('\0');
    let mut lines = Vec::new();
    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        let mut parts = field.splitn(3, '\t');
        let (added, deleted, path) = (
            parts.next().unwrap(),
            parts.next().unwrap(),
            parts.next().unwrap(),
        );
        let path = if path.is_empty() {
            let old = fields.next().unwrap();
            let new = fields.next().unwrap();
            format!("{old}\t{new}")
        } else {
            path.to_owned()
        };
        lines.push(format!("{added}\t{deleted}\t{path}"));
    }
    lines.sort();
    lines
}

fn our_name_status(changes: &Changes) -> Vec<String> {
    let mut lines: Vec<_> = changes
        .entries()
        .map(|entry| match entry.status {
            Status::Renamed => format!(
                "R{:03}\t{}\t{}",
                entry.similarity.unwrap(),
                entry.old_path.as_ref().unwrap(),
                entry.path
            ),
            status => {
                let letter = match status {
                    Status::Added => "A",
                    Status::Modified => "M",
                    Status::Deleted => "D",
                    Status::TypeChanged => "T",
                    Status::Renamed => unreachable!(),
                };
                format!("{letter}\t{}", entry.path)
            }
        })
        .collect();
    lines.sort();
    lines
}

fn our_numstat(changes: &Changes) -> Vec<String> {
    let mut lines: Vec<_> = changes
        .entries()
        .map(|entry| {
            let counts = if entry.binary {
                "-\t-".to_owned()
            } else {
                format!("{}\t{}", entry.additions, entry.deletions)
            };
            match &entry.old_path {
                Some(old) => format!("{counts}\t{old}\t{}", entry.path),
                None => format!("{counts}\t{}", entry.path),
            }
        })
        .collect();
    lines.sort();
    lines
}

/// Applies the allowed differences to git's numstat lines, checking that each
/// one still happens.
fn allow_differences(fixture: &str, git: Vec<String>, ours: &[String]) -> Vec<String> {
    let mut git = git;
    for &(name, path, reason) in ALLOWED_DIFFERENCES {
        if name != fixture {
            continue;
        }
        for line in git.iter_mut() {
            if line.rsplit('\t').next() == Some(path) && !line.starts_with("-\t-") {
                let rest = line.splitn(3, '\t').nth(2).unwrap().to_owned();
                let replaced = format!("-\t-\t{rest}");
                assert!(ours.contains(&replaced), "{path}: {reason}");
                *line = replaced;
            }
        }
    }
    git.sort();
    git
}

/// Compares TurboDiff's changes with `git diff <args>` run in `dir`.
fn assert_matches_git(fixture: &str, changes: &Changes, dir: &Path, args: &[&str]) {
    let git = |extra: &[&str]| {
        let mut all = vec![
            "-c",
            "core.quotePath=false",
            "diff",
            "-z",
            "-M",
            "--no-ext-diff",
            "--no-textconv",
        ];
        all.extend_from_slice(extra);
        all.extend_from_slice(args);
        common::git(dir, &all)
    };
    let context = format!("{fixture}: git diff {}", args.join(" "));
    assert_eq!(
        our_name_status(changes),
        parse_name_status(&git(&["--name-status"])),
        "{context} --name-status"
    );
    let ours = our_numstat(changes);
    let expected = allow_differences(
        fixture,
        parse_numstat(&git(&["--numstat", "--diff-algorithm=histogram"])),
        &ours,
    );
    assert_eq!(ours, expected, "{context} --numstat");
}

fn changes(fixture: &Fixture, comparison: &Comparison) -> Changes {
    Repo::open(&fixture.path)
        .unwrap()
        .changes(comparison, Algorithm::Histogram)
        .unwrap()
}

/// Checks the work tree against `base`, which git sees as the staged copy
/// against `git_base`.
fn check_worktree(name: &str, fixture: &Fixture, comparison: &Comparison, git_base: Option<&str>) {
    let ours = changes(fixture, comparison);
    let staged = fixture.staged_copy();
    let mut args = vec!["--cached"];
    args.extend(git_base);
    assert_matches_git(name, &ours, &staged.path, &args);
}

fn check_commits(name: &str, fixture: &Fixture, comparison: &Comparison, git_args: &[&str]) {
    let ours = changes(fixture, comparison);
    assert_matches_git(name, &ours, &fixture.path, git_args);
}

#[test]
fn worktree_matches_git() {
    let fixture = Fixture::build("worktree");
    check_worktree("worktree", &fixture, &Comparison::default(), Some("HEAD"));
}

#[test]
fn worktree_lists_staged_unstaged_and_untracked_changes_but_not_ignored_files() {
    let fixture = Fixture::build("worktree");
    let ours = changes(&fixture, &Comparison::default());
    let paths: Vec<_> = ours.entries().map(|e| e.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "both.txt",
            "deleted.txt",
            "empty.txt",
            "mixed-whitespace.txt",
            "modified.txt",
            "newdir/sub/file.txt",
            "src/lib.rs",
            "staged-deleted.txt",
            "staged-new.txt",
            "staged.txt",
            "untracked.txt",
            "whitespace.txt",
        ]
    );
    assert_eq!(ours.totals.files, 12);
    let both = ours.entries().find(|e| e.path == "both.txt").unwrap();
    assert_eq!(
        (both.status, both.additions, both.deletions),
        (Status::Modified, 2, 1)
    );
}

#[test]
fn empty_repository_shows_every_file_as_added() {
    let fixture = Fixture::build("empty");
    let ours = changes(&fixture, &Comparison::default());
    assert!(ours.entries().all(|e| e.status == Status::Added));
    check_worktree("empty", &fixture, &Comparison::default(), None);
}

#[test]
fn renames_match_git() {
    let fixture = Fixture::build("renames");
    check_worktree("renames", &fixture, &Comparison::default(), Some("HEAD"));
    check_commits(
        "renames",
        &fixture,
        &commits("HEAD~1", "HEAD", Mode::TwoDot),
        &["HEAD~1", "HEAD"],
    );
}

#[test]
fn binary_mode_and_submodule_changes_match_git() {
    let fixture = Fixture::build("binary");
    check_worktree("binary", &fixture, &Comparison::default(), Some("HEAD"));
    check_commits(
        "binary",
        &fixture,
        &commits("HEAD~1", "HEAD", Mode::TwoDot),
        &["HEAD~1", "HEAD"],
    );
}

#[test]
fn binary_mode_and_submodule_details() {
    let fixture = Fixture::build("binary");
    let ours = changes(&fixture, &commits("HEAD~1", "HEAD", Mode::TwoDot));
    let entry = |path: &str| ours.entries().find(|e| e.path == path).unwrap().clone();

    let script = entry("script.sh");
    assert_eq!(script.mode_change, Some(("100644".into(), "100755".into())));
    assert_eq!((script.additions, script.deletions), (0, 0));
    let image = entry("image.png");
    assert!(image.binary);
    let sub = entry("sub");
    let (old, new) = sub.submodule.unwrap();
    assert_eq!(old, fixture.git(&["rev-parse", "HEAD~1:sub"]).trim());
    assert_eq!(new, fixture.git(&["rev-parse", "HEAD:sub"]).trim());
    assert_eq!(entry("gone.txt").status, Status::Deleted);
}

#[test]
fn line_endings_and_ident_are_filtered_like_git() {
    let fixture = Fixture::build("line_endings");
    let ours = changes(&fixture, &Comparison::default());
    // External filter drivers never run.
    assert!(!fixture.path.join(".git/filter-ran").exists());
    let paths: Vec<_> = ours.entries().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["changed.txt", "driver.dat", "ident-changed.c"]);
    check_worktree(
        "line_endings",
        &fixture,
        &Comparison::default(),
        Some("HEAD"),
    );
}

#[test]
fn three_dot_and_two_dot_match_git() {
    let fixture = Fixture::build("three_dot");
    check_commits(
        "three_dot",
        &fixture,
        &commits("refs/heads/main", "refs/heads/feature", Mode::ThreeDot),
        &["main...feature"],
    );
    check_commits(
        "three_dot",
        &fixture,
        &commits("refs/heads/main", "refs/heads/feature", Mode::TwoDot),
        &["main", "feature"],
    );
    let merge_base = fixture.git(&["merge-base", "main", "HEAD"]);
    check_worktree(
        "three_dot",
        &fixture,
        &worktree("main", Mode::ThreeDot),
        Some(merge_base.trim()),
    );
    check_worktree(
        "three_dot",
        &fixture,
        &worktree("main", Mode::TwoDot),
        Some("main"),
    );
    check_worktree("three_dot", &fixture, &Comparison::default(), Some("HEAD"));
}

#[test]
fn three_dot_hides_what_base_gained_and_two_dot_shows_it_deleted() {
    let fixture = Fixture::build("three_dot");
    let billing = |mode| {
        let ours = changes(&fixture, &commits("main", "feature", mode));
        assert!(!ours.no_merge_base);
        ours.entries()
            .find(|e| e.path == "billing.py")
            .map(|e| e.status)
    };
    assert_eq!(billing(Mode::ThreeDot), None);
    assert_eq!(billing(Mode::TwoDot), Some(Status::Deleted));
}

#[test]
fn unrelated_histories_fall_back_to_two_dot() {
    let fixture = Fixture::build("unrelated");
    let three_dot = changes(&fixture, &commits("main", "other", Mode::ThreeDot));
    assert!(three_dot.no_merge_base);
    let two_dot = changes(&fixture, &commits("main", "other", Mode::TwoDot));
    assert!(!two_dot.no_merge_base);
    assert_eq!(our_numstat(&three_dot), our_numstat(&two_dot));
    assert_matches_git("unrelated", &three_dot, &fixture.path, &["main", "other"]);

    let clean = changes(&fixture, &worktree("other", Mode::ThreeDot));
    assert!(clean.no_merge_base);
}

#[test]
fn hide_whitespace_matches_git_diff_w() {
    let fixture = Fixture::build("worktree");
    let comparison = Comparison {
        ignore_whitespace: true,
        ..Comparison::default()
    };
    let ours = changes(&fixture, &comparison);
    let staged = fixture.staged_copy();
    let git = common::git(
        &staged.path,
        &[
            "diff",
            "-z",
            "-M",
            "-w",
            "--numstat",
            "--diff-algorithm=histogram",
            "--cached",
            "HEAD",
        ],
    );
    // git leaves out files whose only changes are whitespace. TurboDiff
    // keeps them, with no counts, to show "Whitespace-only changes" (PRD §6.7).
    let mut expected = parse_numstat(&git);
    expected.push("0\t0\twhitespace.txt".into());
    expected.sort();
    assert_eq!(our_numstat(&ours), expected);
}

#[test]
fn myers_counts_match_git() {
    let fixture = Fixture::build("worktree");
    let ours = Repo::open(&fixture.path)
        .unwrap()
        .changes(&Comparison::default(), Algorithm::Myers)
        .unwrap();
    let staged = fixture.staged_copy();
    let git = common::git(
        &staged.path,
        &[
            "diff",
            "-z",
            "-M",
            "--numstat",
            "--diff-algorithm=myers",
            "--cached",
            "HEAD",
        ],
    );
    assert_eq!(our_numstat(&ours), parse_numstat(&git));
}

#[test]
fn unknown_refs_are_ref_not_found() {
    let fixture = Fixture::build("three_dot");
    let repo = Repo::open(&fixture.path).unwrap();
    let err = repo
        .changes(
            &commits("main", "no-such-branch", Mode::TwoDot),
            Algorithm::Histogram,
        )
        .err()
        .unwrap();
    assert!(matches!(err, turbodiff_repo::Error::RefNotFound(spec) if spec == "no-such-branch"));
}
