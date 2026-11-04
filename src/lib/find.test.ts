import { describe, expect, it } from 'vitest';
import { buildRows, type LoadedDiff } from './diffRows';
import { findMatches, marksByCell } from './find';
import type { FileDiff, FileEntry } from './types';

function entry(path: string): FileEntry {
  return {
    path,
    oldPath: null,
    status: 'M',
    similarity: null,
    additions: 1,
    deletions: 1,
    binary: false,
    modeChange: null,
    submodule: null,
  };
}

const diff: FileDiff = {
  kind: 'text',
  hunks: [
    {
      oldStart: 1,
      oldLines: 2,
      newStart: 1,
      newLines: 2,
      lines: [
        { kind: 'ctx', oldNo: 1, newNo: 1, text: 'Fifty fifty', words: null },
        { kind: 'del', oldNo: 2, newNo: null, text: 'old FIFTY', words: null },
        { kind: 'add', oldNo: null, newNo: 2, text: 'new fifty', words: null },
      ],
    },
  ],
  oldLineCount: 2,
  newLineCount: 2,
  noNewlineAtEof: { old: false, new: false },
};

function rows(style: 'inline' | 'split', collapsed: string[] = []) {
  const loaded: LoadedDiff = { diff, context: new Map() };
  return buildRows([entry('fifty.txt'), entry('b.txt')], {
    style,
    diffs: () => loaded,
    collapsed: new Set(collapsed),
    context: 3,
  });
}

describe('findMatches', () => {
  it('finds every case-insensitive match in the lines, in order', () => {
    const matches = findMatches(rows('inline'), 'fifty');
    expect(matches.map(({ cell, start }) => [cell, start])).toEqual([
      ['line', 0],
      ['line', 6],
      ['line', 4],
      ['line', 4],
      ['line', 0],
      ['line', 6],
      ['line', 4],
      ['line', 4],
    ]);
    expect(matches[0].key).toBe('fifty.txt\0l1:1');
  });

  it('searches both sides of split rows, left first', () => {
    const matches = findMatches(rows('split'), 'fifty').slice(0, 4);
    expect(matches.map(({ cell, start }) => [cell, start])).toEqual([
      ['left', 0],
      ['left', 6],
      ['right', 0],
      ['right', 6],
    ]);
  });

  it("doesn't search file paths, hunk headers or collapsed files", () => {
    expect(findMatches(rows('inline', ['fifty.txt', 'b.txt']), 'fifty')).toEqual([]);
    expect(findMatches(rows('inline'), '@@')).toEqual([]);
  });

  it("doesn't overlap matches, and finds nothing for an empty query", () => {
    expect(findMatches(rows('inline'), 'ff').length).toBe(0);
    expect(findMatches(rows('inline'), 'if').length).toBe(8);
    expect(findMatches(rows('inline'), '')).toEqual([]);
  });
});

describe('marksByCell', () => {
  it('groups the marks by cell and marks the current match', () => {
    const matches = findMatches(rows('split'), 'fifty');
    const marks = marksByCell(matches, matches[1]);
    expect(marks.get('fifty.txt\0s1:1\0left')).toEqual([
      { start: 0, end: 5, current: false },
      { start: 6, end: 11, current: true },
    ]);
    expect(marks.get('fifty.txt\0s1:1\0right')?.every((mark) => !mark.current)).toBe(true);
  });
});
