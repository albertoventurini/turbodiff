//! Rename detection that follows git's own rules (diffcore-rename.c and
//! diffcore-delta.c), so that borderline cases agree with `git diff -M`.

use std::collections::HashMap;

use gix::ObjectId;
use gix::bstr::{BStr, ByteSlice};

use crate::Result;

/// git's scale for similarity scores: 100% is `MAX_SCORE`.
pub(crate) const MAX_SCORE: u64 = 60_000;
/// The least similarity for a rename, git's default of 50%.
const MIN_SCORE: u64 = MAX_SCORE / 2;
/// Same-named files only need this much similarity to pair up first, as in
/// git: halfway between `MIN_SCORE` and 100%.
const MIN_BASENAME_SCORE: u64 = MIN_SCORE + (MAX_SCORE - MIN_SCORE) / 2;
/// Inexact detection is skipped when there are more than this many sources
/// times destinations squared (git's `diff.renameLimit` default).
const RENAME_LIMIT: usize = 1_000;
/// How many best sources git keeps for each destination.
const CANDIDATES_PER_DEST: usize = 4;

/// A deleted file (a source) or an added file (a destination).
pub(crate) struct Candidate<'a> {
    pub path: &'a BStr,
    pub id: ObjectId,
    /// A regular or executable file. Only these take part in inexact detection.
    pub regular: bool,
}

/// The content of a candidate, as loaded for comparing.
pub(crate) struct Content {
    pub data: Vec<u8>,
    pub binary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Which {
    Source,
    Dest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rename {
    pub source: usize,
    pub dest: usize,
    /// Similarity, from 0 to [`MAX_SCORE`].
    pub score: u64,
}

impl Rename {
    pub fn percent(&self) -> u8 {
        (self.score * 100 / MAX_SCORE) as u8
    }
}

/// Pairs up `sources` with `dests`. `load` gives the content of a candidate
/// and is only called when an inexact comparison needs it.
pub(crate) fn detect(
    sources: &[Candidate<'_>],
    dests: &[Candidate<'_>],
    load: impl FnMut(Which, usize) -> Result<Content>,
) -> Result<Vec<Rename>> {
    let mut detector = Detector {
        sources,
        dests,
        load,
        source_used: vec![false; sources.len()],
        dest_used: vec![false; dests.len()],
        spans: [HashMap::new(), HashMap::new()],
        renames: Vec::new(),
    };
    if sources.is_empty() || dests.is_empty() {
        return Ok(Vec::new());
    }
    detector.exact();
    detector.by_basename()?;
    detector.inexact()?;
    Ok(detector.renames)
}

struct Detector<'c, 'p, F> {
    sources: &'c [Candidate<'p>],
    dests: &'c [Candidate<'p>],
    load: F,
    source_used: Vec<bool>,
    dest_used: Vec<bool>,
    /// Content size and span hashes per candidate, computed on first use.
    spans: [HashMap<usize, Option<Spans>>; 2],
    renames: Vec<Rename>,
}

struct Spans {
    size: u64,
    counts: HashMap<u32, u64>,
}

impl<F: FnMut(Which, usize) -> Result<Content>> Detector<'_, '_, F> {
    fn record(&mut self, source: usize, dest: usize, score: u64) {
        self.source_used[source] = true;
        self.dest_used[dest] = true;
        self.renames.push(Rename {
            source,
            dest,
            score,
        });
    }

    /// Identical content: for each destination, prefer a source with the same
    /// file name.
    fn exact(&mut self) {
        let mut by_id: HashMap<ObjectId, Vec<usize>> = HashMap::new();
        for (i, source) in self.sources.iter().enumerate() {
            by_id.entry(source.id).or_default().push(i);
        }
        for (d, dest) in self.dests.iter().enumerate() {
            let Some(candidates) = by_id.get(&dest.id) else {
                continue;
            };
            let best = candidates
                .iter()
                .copied()
                .filter(|&s| !self.source_used[s] && self.sources[s].regular == dest.regular)
                .max_by_key(|&s| {
                    (
                        basename_same(self.sources[s].path, dest.path),
                        usize::MAX - s,
                    )
                });
            if let Some(s) = best {
                self.record(s, d, MAX_SCORE);
            }
        }
    }

    /// Files whose name is unique among the remaining sources and among the
    /// remaining destinations pair up if they're similar enough.
    fn by_basename(&mut self) -> Result<()> {
        let sources = unique_basenames(self.sources, &self.source_used);
        let dests = unique_basenames(self.dests, &self.dest_used);
        let mut pairs: Vec<_> = sources
            .iter()
            .filter_map(|(name, &s)| dests.get(name).map(|&d| (s, d)))
            .collect();
        pairs.sort_unstable();
        for (s, d) in pairs {
            let score = self.similarity(s, d, MIN_BASENAME_SCORE)?;
            if score >= MIN_BASENAME_SCORE {
                self.record(s, d, score);
            }
        }
        Ok(())
    }

    /// Every remaining destination against every remaining source, best
    /// scores first.
    fn inexact(&mut self) -> Result<()> {
        let sources: Vec<_> = (0..self.sources.len())
            .filter(|&s| !self.source_used[s])
            .collect();
        let dests: Vec<_> = (0..self.dests.len())
            .filter(|&d| !self.dest_used[d])
            .collect();
        if sources.is_empty()
            || dests.is_empty()
            || sources.len().saturating_mul(dests.len()) > RENAME_LIMIT * RENAME_LIMIT
        {
            return Ok(());
        }
        // (score, name score, dest, source)
        let mut matches: Vec<(u64, bool, usize, usize)> = Vec::new();
        for &d in &dests {
            let mut best: Vec<(u64, bool, usize, usize)> = Vec::new();
            for &s in &sources {
                let score = self.similarity(s, d, MIN_SCORE)?;
                if score == 0 {
                    continue;
                }
                best.push((
                    score,
                    basename_same(self.sources[s].path, self.dests[d].path),
                    d,
                    s,
                ));
            }
            best.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.3.cmp(&b.3)));
            best.truncate(CANDIDATES_PER_DEST);
            matches.extend(best);
        }
        matches.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then(b.1.cmp(&a.1))
                .then(a.2.cmp(&b.2))
                .then(a.3.cmp(&b.3))
        });
        for (score, _, d, s) in matches {
            if score < MIN_SCORE {
                break;
            }
            if self.dest_used[d] || self.source_used[s] {
                continue;
            }
            self.record(s, d, score);
        }
        Ok(())
    }

    /// git's `estimate_similarity`: how much of the larger file's content is
    /// copied from the source.
    fn similarity(&mut self, s: usize, d: usize, min_score: u64) -> Result<u64> {
        if !self.sources[s].regular || !self.dests[d].regular {
            return Ok(0);
        }
        self.ensure_spans(Which::Source, s)?;
        self.ensure_spans(Which::Dest, d)?;
        let (Some(source), Some(dest)) = (&self.spans[0][&s], &self.spans[1][&d]) else {
            return Ok(0);
        };
        let max_size = source.size.max(dest.size);
        let delta_size = max_size - source.size.min(dest.size);
        // Files whose size changed this much can't be similar enough.
        if max_size * (MAX_SCORE - min_score) < delta_size * MAX_SCORE {
            return Ok(0);
        }
        if dest.size == 0 {
            return Ok(0);
        }
        let copied: u64 = source
            .counts
            .iter()
            .map(|(hash, &count)| count.min(dest.counts.get(hash).copied().unwrap_or(0)))
            .sum();
        Ok(copied * MAX_SCORE / max_size)
    }

    fn ensure_spans(&mut self, which: Which, i: usize) -> Result<()> {
        let index = match which {
            Which::Source => 0,
            Which::Dest => 1,
        };
        if !self.spans[index].contains_key(&i) {
            let content = (self.load)(which, i)?;
            let spans = Spans {
                size: content.data.len() as u64,
                counts: span_hashes(&content.data, !content.binary),
            };
            self.spans[index].insert(i, Some(spans));
        }
        Ok(())
    }
}

/// git's `hash_chars`: splits content into spans that end at a newline or
/// after 64 bytes, and counts the bytes in the spans with each hash. In text,
/// a CR before a LF is skipped.
pub(crate) fn span_hashes(data: &[u8], is_text: bool) -> HashMap<u32, u64> {
    const HASHBASE: u32 = 107_927;
    let hash = |accum1: u32, accum2: u32| accum1.wrapping_add(accum2.wrapping_mul(0x61)) % HASHBASE;
    let mut counts = HashMap::new();
    let (mut accum1, mut accum2, mut n) = (0u32, 0u32, 0u64);
    for (i, &c) in data.iter().enumerate() {
        if is_text && c == b'\r' && data.get(i + 1) == Some(&b'\n') {
            continue;
        }
        let old1 = accum1;
        accum1 = (accum1 << 7) ^ (accum2 >> 25);
        accum2 = (accum2 << 7) ^ (old1 >> 25);
        accum1 = accum1.wrapping_add(u32::from(c));
        n += 1;
        if n < 64 && c != b'\n' {
            continue;
        }
        *counts.entry(hash(accum1, accum2)).or_insert(0) += n;
        (accum1, accum2, n) = (0, 0, 0);
    }
    if n > 0 {
        *counts.entry(hash(accum1, accum2)).or_insert(0) += n;
    }
    counts
}

fn basename(path: &BStr) -> &BStr {
    path.rsplit_str("/").next().unwrap_or_default().as_bstr()
}

fn basename_same(a: &BStr, b: &BStr) -> bool {
    basename(a) == basename(b)
}

/// File names that appear exactly once among the unused candidates.
fn unique_basenames<'p>(candidates: &[Candidate<'p>], used: &[bool]) -> HashMap<&'p BStr, usize> {
    let mut seen: HashMap<&BStr, Option<usize>> = HashMap::new();
    for (i, candidate) in candidates.iter().enumerate() {
        if used[i] || !candidate.regular {
            continue;
        }
        seen.entry(basename(candidate.path))
            .and_modify(|slot| *slot = None)
            .or_insert(Some(i));
    }
    seen.into_iter()
        .filter_map(|(name, i)| i.map(|i| (name, i)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> ObjectId {
        ObjectId::from_bytes_or_panic(&[n; 20])
    }

    fn run(sources: &[(&str, &str)], dests: &[(&str, &str)]) -> Vec<(usize, usize, u8)> {
        let ids: HashMap<&str, ObjectId> = sources
            .iter()
            .chain(dests)
            .enumerate()
            .map(|(i, (_, data))| (*data, id(i as u8)))
            .collect();
        fn candidates<'a>(
            list: &[(&'a str, &str)],
            ids: &HashMap<&str, ObjectId>,
        ) -> Vec<Candidate<'a>> {
            list.iter()
                .map(|(path, data)| Candidate {
                    path: path.as_bytes().as_bstr(),
                    id: ids[data],
                    regular: true,
                })
                .collect()
        }
        let (s, d) = (candidates(sources, &ids), candidates(dests, &ids));
        let renames = detect(&s, &d, |which, i| {
            let data = match which {
                Which::Source => sources[i].1,
                Which::Dest => dests[i].1,
            };
            Ok(Content {
                data: data.as_bytes().to_vec(),
                binary: false,
            })
        })
        .unwrap();
        let mut out: Vec<_> = renames
            .iter()
            .map(|r| (r.source, r.dest, r.percent()))
            .collect();
        out.sort_unstable();
        out
    }

    fn numbered(prefix: &str, changed: &[u32]) -> String {
        (1..=20)
            .map(|n| {
                if changed.contains(&n) {
                    format!("{} {n}\n", prefix.to_uppercase())
                } else {
                    format!("{prefix} {n}\n")
                }
            })
            .collect()
    }

    #[test]
    fn spans_end_at_newlines_and_skip_cr_before_lf_in_text() {
        let lf = span_hashes(b"one\ntwo\n", true);
        assert_eq!(lf.values().sum::<u64>(), 8);
        assert_eq!(lf, span_hashes(b"one\r\ntwo\r\n", true));
        assert_ne!(lf, span_hashes(b"one\r\ntwo\r\n", false));
        // Long lines are cut every 64 bytes.
        let long = span_hashes(&[b'x'; 130], true);
        assert_eq!(long.values().sum::<u64>(), 130);
        assert_eq!(long.len(), 2);
    }

    #[test]
    fn identical_content_is_a_full_rename_preferring_the_same_file_name() {
        let renames = run(
            &[("a/x.txt", "same\n"), ("b/y.txt", "same\n")],
            &[("c/y.txt", "same\n")],
        );
        assert_eq!(renames, [(1, 0, 100)]);
    }

    #[test]
    fn similarity_matches_git_near_the_threshold() {
        // The same cases as the renames fixture, whose scores git reports as
        // R055 and below 50%.
        let near = run(
            &[("near.txt", &numbered("near", &[]))],
            &[(
                "near-renamed.txt",
                &numbered("near", &[1, 3, 5, 7, 9, 11, 13, 15, 17]),
            )],
        );
        assert_eq!(near, [(0, 0, 55)]);
        let far = run(
            &[("far.txt", &numbered("far", &[]))],
            &[(
                "far-renamed.txt",
                &numbered("far", &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
            )],
        );
        assert!(far.is_empty());
    }

    #[test]
    fn each_source_is_used_once_by_the_best_match() {
        let base = numbered("line", &[]);
        let close = numbered("line", &[1]);
        let further = numbered("line", &[1, 2, 3, 4]);
        let renames = run(
            &[("old.txt", &base)],
            &[("further.txt", &further), ("close.txt", &close)],
        );
        // git reports R095 for this pair.
        assert_eq!(renames, [(0, 1, 95)]);
    }

    #[test]
    fn very_different_sizes_are_not_compared() {
        let small = "line 1\n".to_owned();
        let big = numbered("line", &[]);
        assert!(run(&[("a", &small)], &[("b", &big)]).is_empty());
    }

    #[test]
    fn links_are_only_renamed_when_identical() {
        let sources = [Candidate {
            path: b"link".as_bstr(),
            id: id(1),
            regular: false,
        }];
        let dests = [Candidate {
            path: b"file".as_bstr(),
            id: id(2),
            regular: true,
        }];
        let renames = detect(&sources, &dests, |_, _| unreachable!()).unwrap();
        assert!(renames.is_empty());
    }
}
