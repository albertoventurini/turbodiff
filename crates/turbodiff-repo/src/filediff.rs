//! The diff of one file: hunks, context and word-level ranges (TECH.md §4.5).

use std::sync::Arc;

use imara_diff::{Diff, InternedInput};
use serde::{Deserialize, Serialize};

use crate::changes::ChangedFile;
use crate::content::{Loader, Version};
use crate::linediff::{self, Algorithm, ChangedRange};
use crate::{Error, Repo, Result};

/// A file bigger than this starts collapsed ("Large diff not shown").
/// Provisional, to be tuned on real repositories (PRD open question 1).
pub const LARGE_FILE_BYTES: u64 = 500 * 1024;
/// A diff with more changed lines than this starts collapsed. Provisional,
/// like [`LARGE_FILE_BYTES`].
pub const LARGE_DIFF_LINES: u32 = 1_000;
/// Paired lines with less than this share of their text in common get no
/// word-level ranges, as on GitHub.
const WORD_DIFF_MIN_SIMILARITY: f64 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffOptions {
    pub algorithm: Algorithm,
    pub ignore_whitespace: bool,
    /// Unchanged lines around each change (PRD §6.8).
    pub context: u32,
    /// Show the diff even if the file is large ("Load diff").
    pub force: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Histogram,
            ignore_whitespace: false,
            context: 3,
            force: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileDiff {
    #[serde(rename_all = "camelCase")]
    Text {
        hunks: Vec<Hunk>,
        old_line_count: u32,
        new_line_count: u32,
        no_newline_at_eof: NoNewlineAtEof,
    },
    #[serde(rename_all = "camelCase")]
    Binary {
        old_size: Option<u64>,
        new_size: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    TooLarge {
        additions: u32,
        deletions: u32,
    },
    WhitespaceOnly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, specta::Type)]
pub struct NoNewlineAtEof {
    pub old: bool,
    pub new: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    /// As in a `@@ -old_start,old_lines +new_start,new_lines @@` header. With
    /// no lines on a side, the start is the line before the change.
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
pub enum LineKind {
    #[serde(rename = "ctx")]
    Context,
    #[serde(rename = "add")]
    Added,
    #[serde(rename = "del")]
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub kind: LineKind,
    /// 1-based line numbers.
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    /// The text without its newline.
    pub text: String,
    /// Changed `[start, end)` ranges in UTF-16 code units, for paired changed lines.
    pub words: Option<Vec<(u32, u32)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Old,
    New,
}

impl Repo {
    /// The diff of one file from [`Repo::changes`].
    pub fn file_diff(&self, file: &ChangedFile, options: &DiffOptions) -> Result<FileDiff> {
        let repo = self.local();
        let mut loader = Loader::new(&repo, self.root())?;
        let old = self.load(&mut loader, file.old.as_ref())?;
        let new = self.load(&mut loader, file.new.as_ref())?;
        let size = |data: &Option<Arc<[u8]>>| data.as_ref().map(|data| data.len() as u64);
        if file.entry.binary {
            return Ok(FileDiff::Binary {
                old_size: size(&old),
                new_size: size(&new),
            });
        }
        let too_large = size(&old).max(size(&new)).unwrap_or(0) > LARGE_FILE_BYTES;
        if too_large && !options.force {
            return Ok(FileDiff::TooLarge {
                additions: file.entry.additions,
                deletions: file.entry.deletions,
            });
        }
        let (Ok(old_text), Ok(new_text)) = (decode(&old), decode(&new)) else {
            return Ok(FileDiff::Binary {
                old_size: size(&old),
                new_size: size(&new),
            });
        };
        Ok(diff_texts(old_text, new_text, options))
    }

    /// Lines `start..=end` (1-based, clamped to the file) of one side of a
    /// file, for expanding context.
    pub fn lines(
        &self,
        file: &ChangedFile,
        side: Side,
        start: u32,
        end: u32,
    ) -> Result<Vec<String>> {
        let repo = self.local();
        let mut loader = Loader::new(&repo, self.root())?;
        let version = match side {
            Side::Old => file.old.as_ref(),
            Side::New => file.new.as_ref(),
        };
        let data = self
            .load(&mut loader, version)?
            .unwrap_or_else(|| Arc::from([]));
        let text = String::from_utf8_lossy(&data);
        let start = start.max(1) as usize;
        Ok(text_lines(&text)
            .into_iter()
            .skip(start - 1)
            .take((end as usize + 1).saturating_sub(start))
            .map(str::to_owned)
            .collect())
    }

    /// The content of `version`, through the cache.
    fn load(
        &self,
        loader: &mut Loader<'_>,
        version: Option<&Version>,
    ) -> Result<Option<Arc<[u8]>>> {
        let Some(version) = version else {
            return Ok(None);
        };
        let mut cache = self.cache.lock().map_err(Error::internal)?;
        if let Some(data) = cache.get(&version.id) {
            return Ok(Some(data));
        }
        let data: Arc<[u8]> = loader.load(version)?.into();
        // A work-tree file may have changed since the list was built. Its new
        // content doesn't belong under the old id.
        let current = !version.on_disk
            || gix::objs::compute_hash(loader.object_hash(), gix::object::Kind::Blob, &data)
                .is_ok_and(|id| id == version.id);
        if current {
            cache.insert(version.id, data.clone());
        }
        Ok(Some(data))
    }
}

fn decode(data: &Option<Arc<[u8]>>) -> Result<&str, std::str::Utf8Error> {
    std::str::from_utf8(data.as_deref().unwrap_or_default())
}

/// Lines without their newlines.
fn text_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n')
        .map(|line| line.strip_suffix('\n').unwrap_or(line))
        .collect()
}

fn diff_texts(old: &str, new: &str, options: &DiffOptions) -> FileDiff {
    let old_lines = linediff::split_lines(old.as_bytes());
    let new_lines = linediff::split_lines(new.as_bytes());
    let ranges = linediff::diff_lines(
        &old_lines,
        &new_lines,
        options.algorithm,
        options.ignore_whitespace,
    );
    let (additions, deletions) = linediff::count(&ranges);
    if !options.force && additions + deletions > LARGE_DIFF_LINES {
        return FileDiff::TooLarge {
            additions,
            deletions,
        };
    }
    if ranges.is_empty() && options.ignore_whitespace && old != new {
        return FileDiff::WhitespaceOnly;
    }
    let old_text = text_lines(old);
    let new_text = text_lines(new);
    let mut hunks = build_hunks(&old_text, &new_text, &ranges, options.context);
    for hunk in &mut hunks {
        add_word_ranges(&mut hunk.lines, options.ignore_whitespace);
    }
    FileDiff::Text {
        hunks,
        old_line_count: old_text.len() as u32,
        new_line_count: new_text.len() as u32,
        no_newline_at_eof: NoNewlineAtEof {
            old: !old.is_empty() && !old.ends_with('\n'),
            new: !new.is_empty() && !new.ends_with('\n'),
        },
    }
}

/// Groups changed ranges into hunks with `context` unchanged lines around
/// them. Ranges whose context would touch or overlap share a hunk, as in git.
/// Unchanged lines show the new text, which differs from the old only in
/// whitespace when that's hidden.
fn build_hunks(old: &[&str], new: &[&str], ranges: &[ChangedRange], context: u32) -> Vec<Hunk> {
    let mut groups: Vec<&[ChangedRange]> = Vec::new();
    let mut first = 0;
    for i in 1..=ranges.len() {
        if i == ranges.len() || ranges[i].before.start - ranges[i - 1].before.end > 2 * context {
            groups.push(&ranges[first..i]);
            first = i;
        }
    }
    groups
        .into_iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let (head, tail) = (&group[0], &group[group.len() - 1]);
            let before = context.min(head.before.start);
            let after = context.min(old.len() as u32 - tail.before.end);
            let (old_start, new_start) = (head.before.start - before, head.after.start - before);
            let mut lines = Vec::new();
            let (mut o, mut n) = (old_start, new_start);
            let context_until = |lines: &mut Vec<Line>, o: &mut u32, n: &mut u32, end: u32| {
                while *o < end {
                    lines.push(Line {
                        kind: LineKind::Context,
                        old_no: Some(*o + 1),
                        new_no: Some(*n + 1),
                        text: new[*n as usize].to_owned(),
                        words: None,
                    });
                    *o += 1;
                    *n += 1;
                }
            };
            for range in group {
                context_until(&mut lines, &mut o, &mut n, range.before.start);
                for i in range.before.clone() {
                    lines.push(Line {
                        kind: LineKind::Deleted,
                        old_no: Some(i + 1),
                        new_no: None,
                        text: old[i as usize].to_owned(),
                        words: None,
                    });
                }
                for i in range.after.clone() {
                    lines.push(Line {
                        kind: LineKind::Added,
                        old_no: None,
                        new_no: Some(i + 1),
                        text: new[i as usize].to_owned(),
                        words: None,
                    });
                }
                (o, n) = (range.before.end, range.after.end);
            }
            context_until(&mut lines, &mut o, &mut n, tail.before.end + after);
            let old_lines = o - old_start;
            let new_lines = n - new_start;
            Hunk {
                old_start: if old_lines == 0 {
                    old_start
                } else {
                    old_start + 1
                },
                old_lines,
                new_start: if new_lines == 0 {
                    new_start
                } else {
                    new_start + 1
                },
                new_lines,
                lines,
            }
        })
        .collect()
}

/// Pairs the deleted and inserted lines of each changed block in order and
/// marks the words that differ.
fn add_word_ranges(lines: &mut [Line], ignore_whitespace: bool) {
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind != LineKind::Deleted {
            i += 1;
            continue;
        }
        let deleted = i;
        while i < lines.len() && lines[i].kind == LineKind::Deleted {
            i += 1;
        }
        let added = i;
        while i < lines.len() && lines[i].kind == LineKind::Added {
            i += 1;
        }
        let pairs = (added - deleted).min(i - added);
        for k in 0..pairs {
            let (old, new) = (deleted + k, added + k);
            if let Some((old_words, new_words)) =
                word_ranges(&lines[old].text, &lines[new].text, ignore_whitespace)
            {
                lines[old].words = Some(old_words);
                lines[new].words = Some(new_words);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenClass {
    Word,
    Space,
    Other,
}

fn class(c: char) -> TokenClass {
    if c.is_alphanumeric() || c == '_' {
        TokenClass::Word
    } else if c.is_whitespace() {
        TokenClass::Space
    } else {
        TokenClass::Other
    }
}

/// Splits a line into words, runs of whitespace and single other characters,
/// as byte ranges.
fn tokenize(line: &str) -> Vec<(usize, usize)> {
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut last = None;
    for (start, c) in line.char_indices() {
        let class = class(c);
        let end = start + c.len_utf8();
        match tokens.last_mut() {
            Some(token) if last == Some(class) && class != TokenClass::Other => token.1 = end,
            _ => tokens.push((start, end)),
        }
        last = Some(class);
    }
    tokens
}

/// The changed ranges of a pair of lines, in UTF-16 code units, or `None`
/// when they have too little in common.
fn word_ranges(
    old: &str,
    new: &str,
    ignore_whitespace: bool,
) -> Option<(Utf16Ranges, Utf16Ranges)> {
    let old_tokens = tokenize(old);
    let new_tokens = tokenize(new);
    let key = |line: &'_ str, (start, end): (usize, usize)| -> String {
        let text = &line[start..end];
        if ignore_whitespace && class(text.chars().next().unwrap()) == TokenClass::Space {
            String::new()
        } else {
            text.to_owned()
        }
    };
    let mut input = InternedInput::default();
    input.update_before(old_tokens.iter().map(|&t| key(old, t)));
    input.update_after(new_tokens.iter().map(|&t| key(new, t)));
    let diff = Diff::compute(imara_diff::Algorithm::Histogram, &input);

    let mut old_changed = Vec::new();
    let mut new_changed = Vec::new();
    for hunk in diff.hunks() {
        if !hunk.before.is_empty() {
            old_changed.push((
                old_tokens[hunk.before.start as usize].0,
                old_tokens[hunk.before.end as usize - 1].1,
            ));
        }
        if !hunk.after.is_empty() {
            new_changed.push((
                new_tokens[hunk.after.start as usize].0,
                new_tokens[hunk.after.end as usize - 1].1,
            ));
        }
    }
    let changed_len = |ranges: &[(usize, usize)]| ranges.iter().map(|(s, e)| e - s).sum::<usize>();
    let total = old.len() + new.len();
    if total == 0 {
        return None;
    }
    let common = total - changed_len(&old_changed) - changed_len(&new_changed);
    if (common as f64) / (total as f64) < WORD_DIFF_MIN_SIMILARITY {
        return None;
    }
    if ignore_whitespace {
        old_changed.retain(|&(s, e)| !old[s..e].chars().all(char::is_whitespace));
        new_changed.retain(|&(s, e)| !new[s..e].chars().all(char::is_whitespace));
    }
    Some((
        utf16_ranges(old, &old_changed),
        utf16_ranges(new, &new_changed),
    ))
}

/// `[start, end)` ranges in UTF-16 code units.
type Utf16Ranges = Vec<(u32, u32)>;

/// Converts byte ranges in `text` to UTF-16 code unit ranges, for JavaScript.
fn utf16_ranges(text: &str, ranges: &[(usize, usize)]) -> Utf16Ranges {
    let utf16_offset = |byte: usize| text[..byte].encode_utf16().count() as u32;
    ranges
        .iter()
        .map(|&(start, end)| (utf16_offset(start), utf16_offset(end)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(context: u32) -> DiffOptions {
        DiffOptions {
            context,
            ..DiffOptions::default()
        }
    }

    fn numbered(count: u32, changed: &[u32]) -> String {
        (1..=count)
            .map(|n| {
                if changed.contains(&n) {
                    format!("changed {n}\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect()
    }

    fn text(diff: FileDiff) -> (Vec<Hunk>, NoNewlineAtEof) {
        match diff {
            FileDiff::Text {
                hunks,
                no_newline_at_eof,
                ..
            } => (hunks, no_newline_at_eof),
            other => panic!("expected text, got {other:?}"),
        }
    }

    fn headers(hunks: &[Hunk]) -> Vec<(u32, u32, u32, u32)> {
        hunks
            .iter()
            .map(|h| (h.old_start, h.old_lines, h.new_start, h.new_lines))
            .collect()
    }

    #[test]
    fn hunks_have_context_around_each_change() {
        let (hunks, _) = text(diff_texts(
            &numbered(20, &[]),
            &numbered(20, &[10]),
            &options(3),
        ));
        assert_eq!(headers(&hunks), [(7, 7, 7, 7)]);
        let kinds: Vec<_> = hunks[0].lines.iter().map(|l| l.kind).collect();
        use LineKind::*;
        assert_eq!(
            kinds,
            [
                Context, Context, Context, Deleted, Added, Context, Context, Context
            ]
        );
        assert_eq!(hunks[0].lines[3].old_no, Some(10));
        assert_eq!(hunks[0].lines[4].new_no, Some(10));
        assert_eq!(hunks[0].lines[4].text, "changed 10");
    }

    #[test]
    fn hunks_merge_when_their_context_touches() {
        let old = numbered(30, &[]);
        // Six unchanged lines between: the contexts touch, so one hunk.
        let (hunks, _) = text(diff_texts(&old, &numbered(30, &[5, 12]), &options(3)));
        assert_eq!(headers(&hunks), [(2, 14, 2, 14)]);
        // Seven unchanged lines between: two hunks.
        let (hunks, _) = text(diff_texts(&old, &numbered(30, &[5, 13]), &options(3)));
        assert_eq!(headers(&hunks), [(2, 7, 2, 7), (10, 7, 10, 7)]);
    }

    #[test]
    fn context_is_clipped_at_the_file_edges() {
        let (hunks, _) = text(diff_texts(
            &numbered(4, &[]),
            &numbered(4, &[1, 4]),
            &options(3),
        ));
        assert_eq!(headers(&hunks), [(1, 4, 1, 4)]);
        let (hunks, _) = text(diff_texts(
            &numbered(10, &[]),
            &numbered(10, &[5]),
            &options(0),
        ));
        assert_eq!(headers(&hunks), [(5, 1, 5, 1)]);
    }

    #[test]
    fn an_empty_side_starts_at_the_line_before() {
        let (hunks, _) = text(diff_texts("", "a\nb\n", &options(3)));
        assert_eq!(headers(&hunks), [(0, 0, 1, 2)]);
        let (hunks, _) = text(diff_texts("a\nb\n", "", &options(3)));
        assert_eq!(headers(&hunks), [(1, 2, 0, 0)]);
        let (hunks, _) = text(diff_texts(
            &numbered(10, &[]),
            &format!(
                "{}inserted\n{}",
                numbered(5, &[]),
                numbered(10, &[])
                    .split_inclusive('\n')
                    .skip(5)
                    .collect::<String>()
            ),
            &options(0),
        ));
        assert_eq!(headers(&hunks), [(5, 0, 6, 1)]);
    }

    #[test]
    fn a_missing_final_newline_is_recorded_per_side() {
        let (hunks, eof) = text(diff_texts("a\nb", "a\nb\n", &options(3)));
        assert_eq!(
            eof,
            NoNewlineAtEof {
                old: true,
                new: false
            }
        );
        assert_eq!(headers(&hunks), [(1, 2, 1, 2)]);
        assert_eq!(hunks[0].lines[1].text, "b");
        assert_eq!(hunks[0].lines[2].text, "b");
    }

    #[test]
    fn hidden_whitespace_keeps_the_original_text() {
        let old = "fn main() {\n    x();\n}\n";
        let new = "fn main() {\n\tx();\n  y();\n}\n";
        let options = DiffOptions {
            ignore_whitespace: true,
            ..options(3)
        };
        let (hunks, _) = text(diff_texts(old, new, &options));
        let lines: Vec<_> = hunks[0]
            .lines
            .iter()
            .map(|l| (l.kind, l.text.as_str()))
            .collect();
        use LineKind::*;
        assert_eq!(
            lines,
            [
                (Context, "fn main() {"),
                (Context, "\tx();"),
                (Added, "  y();"),
                (Context, "}")
            ]
        );
    }

    #[test]
    fn whitespace_only_changes_are_reported_when_hidden() {
        let options = DiffOptions {
            ignore_whitespace: true,
            ..options(3)
        };
        assert_eq!(
            diff_texts("a b\n", "a  b\n", &options),
            FileDiff::WhitespaceOnly
        );
        let (hunks, _) = text(diff_texts("a\n", "a\n", &options));
        assert!(hunks.is_empty());
    }

    #[test]
    fn many_changed_lines_are_too_large_unless_forced() {
        let old = numbered(LARGE_DIFF_LINES, &[]);
        let new = numbered(
            LARGE_DIFF_LINES,
            &(1..=LARGE_DIFF_LINES).collect::<Vec<_>>(),
        );
        assert_eq!(
            diff_texts(&old, &new, &options(3)),
            FileDiff::TooLarge {
                additions: LARGE_DIFF_LINES,
                deletions: LARGE_DIFF_LINES
            }
        );
        let forced = DiffOptions {
            force: true,
            ..options(3)
        };
        assert!(matches!(
            diff_texts(&old, &new, &forced),
            FileDiff::Text { .. }
        ));
    }

    #[test]
    fn tokens_are_words_whitespace_runs_and_single_punctuation() {
        let line = "let  x_1 = f(a);";
        let tokens: Vec<_> = tokenize(line)
            .into_iter()
            .map(|(s, e)| &line[s..e])
            .collect();
        assert_eq!(
            tokens,
            ["let", "  ", "x_1", " ", "=", " ", "f", "(", "a", ")", ";"]
        );
    }

    #[test]
    fn paired_lines_get_word_ranges() {
        let (old, new) = word_ranges("let x = compute(a);", "let x = compute(b);", false).unwrap();
        assert_eq!(old, [(16, 17)]);
        assert_eq!(new, [(16, 17)]);
    }

    #[test]
    fn lines_with_little_in_common_get_no_word_ranges() {
        assert_eq!(
            word_ranges("completely different", "nothing alike here!", false),
            None
        );
    }

    #[test]
    fn word_ranges_are_in_utf16_code_units() {
        // "é" is one UTF-16 unit and two bytes; "😀" is two units and four bytes.
        let (old, new) = word_ranges("café 😀 old", "café 😀 new", false).unwrap();
        assert_eq!(old, [(8, 11)]);
        assert_eq!(new, [(8, 11)]);
        assert_eq!(utf16_ranges("é😀x", &[(2, 6), (6, 7)]), [(1, 3), (3, 4)]);
    }

    #[test]
    fn deleted_and_added_lines_pair_in_order() {
        let old = "a = 1\nb = 2\nkeep\n";
        let new = "a = 10\nb = 20\nc = 30\nkeep\n";
        let (hunks, _) = text(diff_texts(old, new, &options(0)));
        let words: Vec<_> = hunks[0].lines.iter().map(|l| l.words.clone()).collect();
        assert_eq!(
            words,
            [
                Some(vec![(4, 5)]),
                Some(vec![(4, 5)]),
                Some(vec![(4, 6)]),
                Some(vec![(4, 6)]),
                None
            ]
        );
    }

    #[test]
    fn hidden_whitespace_is_not_marked_as_a_word_change() {
        let (old, new) = word_ranges("a  = 1", "a = 2", true).unwrap();
        assert_eq!(old, [(5, 6)]);
        assert_eq!(new, [(4, 5)]);
    }

    #[test]
    fn text_lines_drop_newlines() {
        assert_eq!(text_lines("a\nb"), ["a", "b"]);
        assert_eq!(text_lines("a\n"), ["a"]);
        assert!(text_lines("").is_empty());
    }
}
