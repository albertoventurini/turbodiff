mod common;

use common::Fixture;

/// Every fixture script runs and leaves a repository behind.
#[test]
fn every_fixture_builds() {
    for name in [
        "worktree",
        "empty",
        "renames",
        "binary",
        "line_endings",
        "three_dot",
        "unrelated",
        "refs",
    ] {
        let fixture = Fixture::build(name);
        assert!(fixture.path.join(".git").is_dir(), "{name} has no .git");
    }
}
