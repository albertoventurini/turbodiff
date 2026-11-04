mod common;

use common::Fixture;
use turbodiff_repo::{CommitItem, RefKind, Repo};

fn lines(output: &str) -> Vec<String> {
    output.lines().map(str::to_owned).collect()
}

fn shas(commits: &[CommitItem]) -> Vec<String> {
    commits.iter().map(|commit| commit.sha.clone()).collect()
}

#[test]
fn branches_put_the_current_one_first_then_local_then_remote_by_recency() {
    let fixture = Fixture::build("refs");
    let branches = Repo::open(&fixture.path).unwrap().list_branches().unwrap();

    let names: Vec<_> = branches.iter().map(|b| b.name.as_str()).collect();
    // `origin/HEAD` is symbolic and left out.
    assert_eq!(
        names,
        [
            "main",
            "feature/login",
            "old",
            "origin/feature/login",
            "origin/main"
        ]
    );
    let current: Vec<_> = branches.iter().filter(|b| b.current).collect();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].full_name, "refs/heads/main");
    assert_eq!(branches[3].kind, RefKind::RemoteBranch);
    assert_eq!(branches[3].full_name, "refs/remotes/origin/feature/login");

    // Within each group, the order matches git's.
    let local = lines(&fixture.git(&[
        "for-each-ref",
        "--sort=-committerdate",
        "--format=%(refname:short) %(objectname) %(committerdate:unix)",
        "refs/heads",
    ]));
    let ours: Vec<_> = branches
        .iter()
        .filter(|b| b.kind == RefKind::LocalBranch && !b.current)
        .map(|b| format!("{} {} {}", b.name, b.commit, b.time))
        .collect();
    let expected: Vec<_> = local
        .into_iter()
        .filter(|line| !line.starts_with("main "))
        .collect();
    assert_eq!(ours, expected);
}

#[test]
fn branches_mark_nothing_current_when_head_is_detached() {
    let fixture = Fixture::build("refs");
    fixture.git(&["switch", "-q", "--detach", "HEAD~1"]);
    let branches = Repo::open(&fixture.path).unwrap().list_branches().unwrap();
    assert!(branches.iter().all(|b| !b.current));
    assert_eq!(branches[0].name, "feature/login");
}

#[test]
fn a_repository_without_commits_has_no_branches_tags_or_commits() {
    let fixture = Fixture::build("empty");
    let repo = Repo::open(&fixture.path).unwrap();
    assert!(repo.list_branches().unwrap().is_empty());
    assert!(repo.list_tags().unwrap().is_empty());
    assert!(repo.list_commits(0, 50).unwrap().is_empty());
    assert!(repo.search_commits("a", 50).unwrap().is_empty());
}

#[test]
fn tags_are_newest_first_by_tagger_or_commit_date() {
    let fixture = Fixture::build("refs");
    let tags = Repo::open(&fixture.path).unwrap().list_tags().unwrap();
    let ours: Vec<_> = tags
        .iter()
        .map(|t| format!("{} {}", t.name, t.time))
        .collect();
    // creatordate is the tagger date for annotated tags and the commit date
    // for lightweight ones.
    let expected = lines(&fixture.git(&[
        "for-each-ref",
        "--sort=-creatordate",
        "--format=%(refname:short) %(creatordate:unix)",
        "refs/tags",
    ]));
    assert_eq!(ours, expected);
    assert_eq!(tags[0].name, "v0.3");
    assert!(tags.iter().all(|t| t.kind == RefKind::Tag));

    // Annotated tags are peeled to their commit.
    let v03 = &tags[0];
    assert_eq!(
        v03.commit,
        fixture.git(&["rev-parse", "v0.3^{commit}"]).trim()
    );
    assert_eq!(v03.full_name, "refs/tags/v0.3");
}

#[test]
fn commits_are_paged_from_head_newest_first() {
    let fixture = Fixture::build("refs");
    let repo = Repo::open(&fixture.path).unwrap();

    let first = repo.list_commits(0, 50).unwrap();
    assert_eq!(
        shas(&first),
        lines(&fixture.git(&["log", "-n", "50", "--format=%H"]))
    );
    let last = repo.list_commits(100, 50).unwrap();
    assert_eq!(
        shas(&last),
        lines(&fixture.git(&["log", "--skip=100", "-n", "50", "--format=%H"]))
    );
    assert_eq!(last.len(), 20);
    assert!(repo.list_commits(500, 50).unwrap().is_empty());

    let head = &first[0];
    let expected = fixture.git(&["log", "-1", "--format=%H%n%s%n%an%n%ct"]);
    assert_eq!(
        format!(
            "{}\n{}\n{}\n{}\n",
            head.sha, head.subject, head.author_name, head.time
        ),
        expected
    );
    assert_eq!(head.short_sha, &head.sha[..7]);
}

#[test]
fn commits_are_searched_by_message_ignoring_case() {
    let fixture = Fixture::build("refs");
    let repo = Repo::open(&fixture.path).unwrap();

    let found = repo.search_commits("FIX bug", 50).unwrap();
    let expected = lines(&fixture.git(&["log", "-i", "--grep=fix bug", "--format=%H"]));
    assert_eq!(expected.len(), 12);
    assert_eq!(shas(&found), expected);

    let limited = repo.search_commits("fix bug", 5).unwrap();
    assert_eq!(shas(&limited), expected[..5]);

    assert!(
        repo.search_commits("no such message", 50)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn commit_prefixes_resolve_to_every_matching_commit() {
    let fixture = Fixture::build("refs");
    let repo = Repo::open(&fixture.path).unwrap();

    let head = fixture.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let found = repo.resolve_commit_prefix(&head).unwrap();
    assert_eq!(shas(&found), std::slice::from_ref(&head));
    let found = repo
        .resolve_commit_prefix(&head[..10].to_uppercase())
        .unwrap();
    assert_eq!(shas(&found), std::slice::from_ref(&head));

    // The fixture has two commits that share their first four hex digits.
    let ambiguous = fixture.git(&["rev-parse", "refs/fixture/ambiguous"]);
    let prefix = &ambiguous[..4];
    let mut expected: Vec<_> =
        lines(&fixture.git(&["rev-parse", &format!("--disambiguate={prefix}")]))
            .into_iter()
            .filter(|sha| fixture.git(&["cat-file", "-t", sha]).trim() == "commit")
            .collect();
    expected.sort();
    let mut found = shas(&repo.resolve_commit_prefix(prefix).unwrap());
    found.sort();
    assert!(
        found.len() >= 2,
        "expected an ambiguous prefix, got {found:?}"
    );
    assert_eq!(found, expected);
}

#[test]
fn text_that_is_not_a_sha_prefix_resolves_to_nothing() {
    let fixture = Fixture::build("refs");
    let repo = Repo::open(&fixture.path).unwrap();
    let head = fixture.git(&["rev-parse", "HEAD"]);
    for text in ["", "abc", "main", "xyz1", &head[..3], &"a".repeat(41)] {
        assert!(
            repo.resolve_commit_prefix(text).unwrap().is_empty(),
            "{text:?}"
        );
    }
    // A tree's SHA is not a commit.
    let tree = fixture.git(&["rev-parse", "HEAD^{tree}"]);
    assert!(repo.resolve_commit_prefix(tree.trim()).unwrap().is_empty());
}
