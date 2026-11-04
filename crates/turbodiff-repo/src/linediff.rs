//! Line diffs with imara-diff (TECH.md §4.5, steps 5–7).

use std::borrow::Cow;

use imara_diff::{Diff, InternedInput};
use serde::{Deserialize, Serialize};

pub use imara_diff::Hunk as ChangedRange;

/// The diff algorithm, chosen in Settings (PRD §6.8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Algorithm {
    #[default]
    Histogram,
    Myers,
}

impl From<Algorithm> for imara_diff::Algorithm {
    fn from(algorithm: Algorithm) -> Self {
        match algorithm {
            Algorithm::Histogram => Self::Histogram,
            Algorithm::Myers => Self::Myers,
        }
    }
}

/// Splits `data` into lines, each with its `\n` if it has one. A missing final
/// newline leaves the last line without one, so it differs from the same text
/// with a newline, as in git.
pub(crate) fn split_lines(data: &[u8]) -> Vec<&[u8]> {
    data.split_inclusive(|&b| b == b'\n').collect()
}

/// What two lines are compared by. With `ignore_whitespace`, all whitespace is
/// removed first, as `git diff -w` does.
fn line_key(line: &[u8], ignore_whitespace: bool) -> Cow<'_, [u8]> {
    if ignore_whitespace {
        Cow::Owned(
            line.iter()
                .copied()
                .filter(|b| !b.is_ascii_whitespace() && *b != b'\x0b')
                .collect(),
        )
    } else {
        Cow::Borrowed(line)
    }
}

/// Above this many lines, both sides together, Histogram gives way to Myers.
/// imara-diff's Histogram is quadratic at worst: a lock file with every other
/// line changed (100,000 lines) took 9 s, against 2 ms for Myers. At this
/// size the worst case stays under half a second (TECH.md §4.5).
pub(crate) const HISTOGRAM_MAX_LINES: usize = 20_000;

/// The ranges of lines that changed between `old` and `new`, in order.
pub(crate) fn diff_lines(
    old: &[&[u8]],
    new: &[&[u8]],
    algorithm: Algorithm,
    ignore_whitespace: bool,
) -> Vec<ChangedRange> {
    let algorithm = if old.len() + new.len() > HISTOGRAM_MAX_LINES {
        Algorithm::Myers
    } else {
        algorithm
    };
    let mut input = InternedInput::default();
    input.update_before(old.iter().map(|line| line_key(line, ignore_whitespace)));
    input.update_after(new.iter().map(|line| line_key(line, ignore_whitespace)));
    let mut diff = Diff::compute(algorithm.into(), &input);
    if ignore_whitespace {
        diff.postprocess_no_heuristic(&input);
    } else {
        diff.postprocess_lines(&input);
    }
    diff.hunks().collect()
}

/// The number of added and deleted lines in `ranges`.
pub(crate) fn count(ranges: &[ChangedRange]) -> (u32, u32) {
    ranges.iter().fold((0, 0), |(added, deleted), range| {
        (
            added + range.after.len() as u32,
            deleted + range.before.len() as u32,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(old: &str, new: &str, ignore_whitespace: bool) -> (u32, u32) {
        let old = split_lines(old.as_bytes());
        let new = split_lines(new.as_bytes());
        count(&diff_lines(
            &old,
            &new,
            Algorithm::Histogram,
            ignore_whitespace,
        ))
    }

    #[test]
    fn lines_keep_their_newlines() {
        assert_eq!(split_lines(b"a\nb\n"), [&b"a\n"[..], b"b\n"]);
        assert_eq!(split_lines(b"a\nb"), [&b"a\n"[..], b"b"]);
        assert!(split_lines(b"").is_empty());
    }

    #[test]
    fn counts_added_and_deleted_lines() {
        assert_eq!(counts("a\nb\nc\n", "a\nB\nc\nd\n", false), (2, 1));
        assert_eq!(counts("", "a\n", false), (1, 0));
        assert_eq!(counts("a\n", "", false), (0, 1));
    }

    #[test]
    fn a_missing_final_newline_changes_the_last_line() {
        assert_eq!(counts("a\nb", "a\nb\n", false), (1, 1));
    }

    #[test]
    fn hiding_whitespace_ignores_whitespace_only_changes() {
        let old = "fn main() {\n    x();\n}\n";
        let new = "fn main() {\n\tx( );\r\n}\n";
        assert_eq!(counts(old, new, false), (1, 1));
        assert_eq!(counts(old, new, true), (0, 0));
        assert_eq!(counts("a b\n", "ab\nc\n", true), (1, 0));
    }

    #[test]
    fn large_files_dont_hit_histograms_worst_case() {
        // Every other line changed, and every line unique: about 3 s with
        // Histogram, a few milliseconds with Myers.
        let version = |minor: u32| {
            (0..30_000)
                .map(|i| format!("package-{i}@1.{minor}.0:\n  resolved {i}\n"))
                .collect::<String>()
        };
        let (old, new) = (version(0), version(1));
        let start = std::time::Instant::now();
        assert_eq!(counts(&old, &new, false), (30_000, 30_000));
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn both_algorithms_find_the_same_counts_for_a_simple_change() {
        let old = split_lines(b"a\nb\nc\nd\n");
        let new = split_lines(b"a\nc\nd\ne\n");
        for algorithm in [Algorithm::Histogram, Algorithm::Myers] {
            assert_eq!(count(&diff_lines(&old, &new, algorithm, false)), (1, 1));
        }
    }
}
