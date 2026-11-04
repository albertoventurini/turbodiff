// Find in page (PRD §6.10). The virtual list only renders rows near the
// viewport, so the webview's own find can't see the rest: matches are found in
// the rows instead. Only the lines of the diffs are searched, as shown: hunk
// headers, messages and the bodies of collapsed files are left out.
import type { Row } from './diffRows';

/** Where a row shows code: an inline line, or one side of a split row. */
export type FindCell = 'line' | 'left' | 'right';

/** A match of the query: `[start, end)` in UTF-16 units of a cell's text. */
export type FindMatch = { key: string; cell: FindCell; start: number; end: number };

/** A match within one cell, as `spans()` marks it. */
export type Mark = { start: number; end: number; current: boolean };

/** Every case-insensitive, non-overlapping match of `query`, in row order. */
export function findMatches(rows: readonly Row[], query: string): FindMatch[] {
  if (query === '') return [];
  const needle = query.toLowerCase();
  const matches: FindMatch[] = [];
  const search = (key: string, cell: FindCell, text: string) => {
    const haystack = text.toLowerCase();
    // Lower-casing can change the length of a few characters; fall back to
    // the text as it is rather than mark the wrong range.
    const same = haystack.length === text.length;
    const source = same ? haystack : text;
    const target = same ? needle : query;
    for (
      let at = source.indexOf(target);
      at !== -1;
      at = source.indexOf(target, at + target.length)
    ) {
      matches.push({ key, cell, start: at, end: at + target.length });
    }
  };
  for (const row of rows) {
    if (row.kind === 'line') search(row.key, 'line', row.line.text);
    else if (row.kind === 'split') {
      if (row.left) search(row.key, 'left', row.left.text);
      if (row.right) search(row.key, 'right', row.right.text);
    }
  }
  return matches;
}

/** Whether two matches are the same place. */
export function sameMatch(a: FindMatch, b: FindMatch): boolean {
  return a.key === b.key && a.cell === b.cell && a.start === b.start;
}

/** The marks of each cell, by row key and cell, with `current` marked as such. */
export function marksByCell(
  matches: readonly FindMatch[],
  current: FindMatch | null,
): Map<string, Mark[]> {
  const marks = new Map<string, Mark[]>();
  for (const match of matches) {
    const id = `${match.key}\0${match.cell}`;
    const list = marks.get(id) ?? [];
    list.push({
      start: match.start,
      end: match.end,
      current: current !== null && sameMatch(match, current),
    });
    marks.set(id, list);
  }
  return marks;
}
