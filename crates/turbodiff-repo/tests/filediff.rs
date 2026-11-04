mod common;

use common::Fixture;
use turbodiff_repo::{
    Algorithm, ChangedFile, Changes, CompareSide, Comparison, DiffOptions, FileDiff,
    LARGE_FILE_BYTES, LineKind, Mode, Repo, Side,
};

fn changes(repo: &Repo, comparison: &Comparison) -> Changes {
    repo.changes(comparison, Algorithm::Histogram).unwrap()
}

fn file<'a>(changes: &'a Changes, path: &str) -> &'a ChangedFile {
    changes
        .files
        .iter()
        .find(|file| file.entry.path == path)
        .unwrap_or_else(|| panic!("{path} is not in the changes"))
}

fn text_hunks(diff: FileDiff) -> Vec<turbodiff_repo::Hunk> {
    match diff {
        FileDiff::Text { hunks, .. } => hunks,
        other => panic!("expected text, got {other:?}"),
    }
}

#[test]
fn a_work_tree_edit_has_hunks_with_context() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(&repo, &Comparison::default());
    let diff = repo
        .file_diff(file(&changes, "src/lib.rs"), &DiffOptions::default())
        .unwrap();
    let FileDiff::Text {
        hunks,
        old_line_count,
        new_line_count,
        no_newline_at_eof,
    } = diff
    else {
        panic!("expected text");
    };
    assert_eq!((old_line_count, new_line_count), (30, 31));
    assert!(!no_newline_at_eof.old && !no_newline_at_eof.new);
    let headers: Vec<_> = hunks
        .iter()
        .map(|h| (h.old_start, h.old_lines, h.new_start, h.new_lines))
        .collect();
    assert_eq!(headers, [(7, 7, 7, 6), (18, 6, 17, 8)]);
    let deleted: Vec<_> = hunks[0]
        .lines
        .iter()
        .filter(|l| l.kind == LineKind::Deleted)
        .map(|l| (l.old_no, l.text.as_str()))
        .collect();
    assert_eq!(deleted, [(Some(10), "fn 10")]);
}

#[test]
fn added_and_deleted_files_are_all_one_kind_of_line() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(&repo, &Comparison::default());
    let options = DiffOptions::default();

    let added = text_hunks(
        repo.file_diff(file(&changes, "untracked.txt"), &options)
            .unwrap(),
    );
    assert_eq!((added[0].old_start, added[0].old_lines), (0, 0));
    assert!(added[0].lines.iter().all(|l| l.kind == LineKind::Added));
    let deleted = text_hunks(
        repo.file_diff(file(&changes, "deleted.txt"), &options)
            .unwrap(),
    );
    assert_eq!((deleted[0].new_start, deleted[0].new_lines), (0, 0));
    assert!(deleted[0].lines.iter().all(|l| l.kind == LineKind::Deleted));
    let empty = text_hunks(
        repo.file_diff(file(&changes, "empty.txt"), &options)
            .unwrap(),
    );
    assert!(empty.is_empty());
}

#[test]
fn whitespace_only_files_are_reported_when_whitespace_is_hidden() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    let comparison = Comparison {
        ignore_whitespace: true,
        ..Comparison::default()
    };
    let changes = changes(&repo, &comparison);
    let options = DiffOptions {
        ignore_whitespace: true,
        ..DiffOptions::default()
    };
    assert_eq!(
        repo.file_diff(file(&changes, "whitespace.txt"), &options)
            .unwrap(),
        FileDiff::WhitespaceOnly
    );
    let mixed = text_hunks(
        repo.file_diff(file(&changes, "mixed-whitespace.txt"), &options)
            .unwrap(),
    );
    let changed: Vec<_> = mixed[0]
        .lines
        .iter()
        .filter(|l| l.kind != LineKind::Context)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(changed, ["mixed 5"]);
    // Unchanged lines keep their new text, tabs included.
    assert_eq!(mixed[0].lines[0].text, "mixed\t2");
}

#[test]
fn binary_and_non_utf8_files_are_binary_with_sizes() {
    let fixture = Fixture::build("binary");
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(
        &repo,
        &Comparison {
            base: "HEAD~1".into(),
            compare: CompareSide::Commit {
                spec: "HEAD".into(),
            },
            mode: Mode::TwoDot,
            ignore_whitespace: false,
        },
    );
    let options = DiffOptions::default();
    assert_eq!(
        repo.file_diff(file(&changes, "image.png"), &options)
            .unwrap(),
        FileDiff::Binary {
            old_size: Some(13),
            new_size: Some(21)
        }
    );
    assert_eq!(
        repo.file_diff(file(&changes, "new.bin"), &options).unwrap(),
        FileDiff::Binary {
            old_size: None,
            new_size: Some(7)
        }
    );
    assert!(matches!(
        repo.file_diff(file(&changes, "latin1.txt"), &options)
            .unwrap(),
        FileDiff::Binary { .. }
    ));
    // A submodule shows its commits as git does.
    let sub = text_hunks(repo.file_diff(file(&changes, "sub"), &options).unwrap());
    let texts: Vec<_> = sub[0].lines.iter().map(|l| l.text.clone()).collect();
    let old = fixture.git(&["rev-parse", "HEAD~1:sub"]);
    let new = fixture.git(&["rev-parse", "HEAD:sub"]);
    assert_eq!(
        texts,
        [
            format!("Subproject commit {}", old.trim()),
            format!("Subproject commit {}", new.trim())
        ]
    );
}

#[test]
fn filtered_content_has_no_carriage_returns() {
    let fixture = Fixture::build("line_endings");
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(&repo, &Comparison::default());
    let hunks = text_hunks(
        repo.file_diff(file(&changes, "changed.txt"), &DiffOptions::default())
            .unwrap(),
    );
    assert!(hunks[0].lines.iter().all(|l| !l.text.contains('\r')));
    assert!(!fixture.path.join(".git/filter-ran").exists());
}

#[test]
fn large_files_are_too_large_unless_forced() {
    let fixture = Fixture::build("worktree");
    let line = "a line of text that is long enough\n";
    let count = LARGE_FILE_BYTES as usize / line.len() + 1;
    std::fs::write(fixture.path.join("big.txt"), line.repeat(count)).unwrap();
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(&repo, &Comparison::default());
    let big = file(&changes, "big.txt");
    assert_eq!(big.entry.additions as usize, count);
    assert_eq!(
        repo.file_diff(big, &DiffOptions::default()).unwrap(),
        FileDiff::TooLarge {
            additions: count as u32,
            deletions: 0
        }
    );
    let forced = DiffOptions {
        force: true,
        ..DiffOptions::default()
    };
    let hunks = text_hunks(repo.file_diff(big, &forced).unwrap());
    assert_eq!(hunks[0].new_lines as usize, count);
}

#[test]
fn lines_come_from_either_side_and_are_clamped() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(&repo, &Comparison::default());
    let lib = file(&changes, "src/lib.rs");
    assert_eq!(
        repo.lines(lib, Side::Old, 9, 11).unwrap(),
        ["fn 9", "fn 10", "fn 11"]
    );
    assert_eq!(
        repo.lines(lib, Side::New, 9, 11).unwrap(),
        ["fn 9", "fn 11", "fn 12"]
    );
    assert_eq!(
        repo.lines(lib, Side::New, 30, 40).unwrap(),
        ["fn 29", "fn 30"]
    );
    assert!(repo.lines(lib, Side::New, 50, 60).unwrap().is_empty());
    let added = file(&changes, "untracked.txt");
    assert!(repo.lines(added, Side::Old, 1, 3).unwrap().is_empty());
}

#[test]
fn contents_are_cached_once_loaded() {
    let fixture = Fixture::build("worktree");
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(&repo, &Comparison::default());
    let untracked = file(&changes, "untracked.txt");
    repo.file_diff(untracked, &DiffOptions::default()).unwrap();
    // Expanding context doesn't read the file again.
    std::fs::remove_file(fixture.path.join("untracked.txt")).unwrap();
    assert_eq!(
        repo.lines(untracked, Side::New, 1, 2).unwrap(),
        ["untracked 1", "untracked 2"]
    );
}

#[test]
fn renamed_files_diff_their_two_versions() {
    let fixture = Fixture::build("renames");
    let repo = Repo::open(&fixture.path).unwrap();
    let changes = changes(
        &repo,
        &Comparison {
            base: "HEAD~1".into(),
            compare: CompareSide::Commit {
                spec: "HEAD".into(),
            },
            mode: Mode::TwoDot,
            ignore_whitespace: false,
        },
    );
    let options = DiffOptions::default();
    let pure = text_hunks(
        repo.file_diff(file(&changes, "moved/pure.txt"), &options)
            .unwrap(),
    );
    assert!(pure.is_empty());
    let edited = text_hunks(
        repo.file_diff(file(&changes, "edited-renamed.txt"), &options)
            .unwrap(),
    );
    let changed: Vec<_> = edited
        .iter()
        .flat_map(|h| &h.lines)
        .filter(|l| l.kind != LineKind::Context)
        .map(|l| (l.kind, l.text.as_str(), l.words.clone()))
        .collect();
    assert_eq!(
        changed,
        [
            (LineKind::Deleted, "edited 3", Some(vec![(7, 8)])),
            (LineKind::Added, "edited three", Some(vec![(7, 12)])),
            (LineKind::Deleted, "edited 15", Some(vec![(7, 9)])),
            (LineKind::Added, "edited fifteen", Some(vec![(7, 14)])),
        ]
    );
}
