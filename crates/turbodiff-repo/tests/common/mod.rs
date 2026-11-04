//! Builds fixture repositories with the scripts in `tests/fixtures/` (TECH.md §10.1)
//! and runs `git` against them to compare results.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// `tests/fixtures/` at the repository root. Found by walking up, because
/// `src-tauri`'s tests use this module too.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|dir| dir.join("tests/fixtures"))
        .find(|dir| dir.join("lib.sh").is_file())
        .expect("tests/fixtures not found")
}

/// A fixture repository in a temporary directory, removed when dropped.
pub struct Fixture {
    _dir: TempDir,
    pub path: PathBuf,
}

impl Fixture {
    /// Runs `tests/fixtures/<name>.sh` to build the repository.
    pub fn build(name: &str) -> Self {
        let dir = tempfile::tempdir().expect("failed to create a temporary directory");
        let path = dir.path().join(name);
        let script = fixtures().join(format!("{name}.sh"));
        let output = Command::new("bash")
            .arg(&script)
            .arg(&path)
            .current_dir(dir.path())
            .output()
            .expect("failed to run bash");
        assert!(
            output.status.success(),
            "{} failed:\n{}",
            script.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        Self { _dir: dir, path }
    }

    /// Runs `git` in the fixture and returns its standard output.
    pub fn git(&self, args: &[&str]) -> String {
        git(&self.path, args)
    }

    /// A copy of the fixture with every work-tree change staged, so that
    /// `git diff --cached` shows what TurboDiff shows for the work tree:
    /// staged, unstaged and untracked changes together.
    pub fn staged_copy(&self) -> Self {
        let dir = tempfile::tempdir().expect("failed to create a temporary directory");
        let path = dir.path().join("copy");
        let status = Command::new("cp")
            .arg("-a")
            .arg(&self.path)
            .arg(&path)
            .status()
            .expect("failed to run cp");
        assert!(status.success(), "cp failed");
        git(&path, &["add", "-A"]);
        Self { _dir: dir, path }
    }
}

/// Runs `git` in `dir` with the same fixed environment as the fixture scripts.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("LC_ALL", "C")
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git {} failed:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("git printed invalid UTF-8")
}
