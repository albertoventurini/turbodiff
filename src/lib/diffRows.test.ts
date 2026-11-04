import { describe, expect, it } from 'vitest';
import {
  buildRows,
  estimateLines,
  expandRange,
  hunkHeader,
  segments,
  type LoadedDiff,
  type Row,
  type RowOptions,
} from './diffRows';
import type { FileDiff, FileEntry, Hunk, Line } from './types';

function entry(path: string, overrides: Partial<FileEntry> = {}): FileEntry {
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
    ...overrides,
  };
}

const ctx = (oldNo: number, newNo: number, text = `line ${newNo}`): Line => ({
  kind: 'ctx',
  oldNo,
  newNo,
  text,
  words: null,
});
const del = (oldNo: number, text: string, words: [number, number][] | null = null): Line => ({
  kind: 'del',
  oldNo,
  newNo: null,
  text,
  words,
});
const add = (newNo: number, text: string, words: [number, number][] | null = null): Line => ({
  kind: 'add',
  oldNo: null,
  newNo,
  text,
  words,
});

function text(hunks: Hunk[], counts: [number, number], eof = { old: false, new: false }): FileDiff {
  return {
    kind: 'text',
    hunks,
    oldLineCount: counts[0],
    newLineCount: counts[1],
    noNewlineAtEof: eof,
  };
}

/** Line 50 of 100 changed, with one line of context. */
const middle = text(
  [
    {
      oldStart: 49,
      oldLines: 3,
      newStart: 49,
      newLines: 3,
      lines: [
        ctx(49, 49),
        del(50, 'old fifty', [[0, 3]]),
        add(50, 'new fifty', [[0, 3]]),
        ctx(51, 51),
      ],
    },
  ],
  [100, 100],
);

function rows(
  files: FileEntry[],
  diffs: Record<string, FileDiff | LoadedDiff | 'failed'>,
  options: Partial<RowOptions> = {},
): Row[] {
  return buildRows(files, {
    style: 'inline',
    diffs: (path) => {
      const value = diffs[path];
      if (value === undefined || value === 'failed' || 'diff' in value) return value;
      return { diff: value, context: new Map() };
    },
    collapsed: new Set(),
    context: 3,
    ...options,
  });
}

/** A compact picture of the rows, for comparing. */
function picture(rows: Row[]): string[] {
  return rows.map((row) => {
    switch (row.kind) {
      case 'header':
        return `header ${row.file.path}${row.collapsed ? ' (collapsed)' : ''}`;
      case 'hunk':
        return `hunk ${row.header ?? '-'} [${row.gap ? `${row.gap.start}-${row.gap.end}` : ''}] ${row.directions.join(',')}`;
      case 'line':
        return `${row.line.kind} ${row.line.oldNo ?? '.'} ${row.line.newNo ?? '.'} ${row.line.text}${row.line.noNewline ? ' ⊘' : ''}`;
      case 'split': {
        const cell = (c: typeof row.left) =>
          c ? `${c.kind} ${c.no} ${c.text}${c.noNewline ? ' ⊘' : ''}` : '(filler)';
        return `${cell(row.left)} | ${cell(row.right)}`;
      }
      case 'message':
        return `message ${JSON.stringify(row.message)}`;
      case 'placeholder':
        return `placeholder ${row.lines}`;
      case 'end':
        return row.closed ? 'end' : 'end (open)';
    }
  });
}

describe('hunkHeader', () => {
  it('leaves out counts of 1, as git does', () => {
    expect(hunkHeader({ oldStart: 1, oldLines: 3, newStart: 1, newLines: 4, lines: [] })).toBe(
      '@@ -1,3 +1,4 @@',
    );
    expect(hunkHeader({ oldStart: 0, oldLines: 0, newStart: 1, newLines: 1, lines: [] })).toBe(
      '@@ -0,0 +1 @@',
    );
  });
});

describe('segments', () => {
  it('marks the changed ranges', () => {
    expect(
      segments('let x = 1;', [
        [4, 5],
        [8, 9],
      ]),
    ).toEqual([
      { text: 'let ', changed: false },
      { text: 'x', changed: true },
      { text: ' = ', changed: false },
      { text: '1', changed: true },
      { text: ';', changed: false },
    ]);
  });

  it('keeps the text whole without ranges', () => {
    expect(segments('abc', null)).toEqual([{ text: 'abc', changed: false }]);
    expect(segments('abc', [])).toEqual([{ text: 'abc', changed: false }]);
  });

  it('uses UTF-16 offsets', () => {
    expect(segments('é😀x', [[1, 3]])).toEqual([
      { text: 'é', changed: false },
      { text: '😀', changed: true },
      { text: 'x', changed: false },
    ]);
  });
});

describe('buildRows, inline', () => {
  it('builds a header, hunk rows, lines and the card end', () => {
    expect(picture(rows([entry('a.txt')], { 'a.txt': middle }))).toEqual([
      'header a.txt',
      'hunk @@ -49,3 +49,3 @@ [1-48] up',
      'ctx 49 49 line 49',
      'del 50 . old fifty',
      'add . 50 new fifty',
      'ctx 51 51 line 51',
      'hunk - [52-100] down',
      'end',
    ]);
  });

  it('keeps word-level ranges on the lines', () => {
    const lines = rows([entry('a.txt')], { 'a.txt': middle }).filter((row) => row.kind === 'line');
    expect(lines.map((row) => row.line.words)).toEqual([null, [[0, 3]], [[0, 3]], null]);
  });

  it('gives a hunk at the top of the file a header with nothing to expand', () => {
    const added = text(
      [{ oldStart: 0, oldLines: 0, newStart: 1, newLines: 2, lines: [add(1, 'a'), add(2, 'b')] }],
      [0, 2],
    );
    expect(picture(rows([entry('new.txt', { status: 'A' })], { 'new.txt': added }))).toEqual([
      'header new.txt',
      'hunk @@ -0,0 +1,2 @@ [] ',
      'add . 1 a',
      'add . 2 b',
      'end',
    ]);
  });

  it('offers to expand all of a small gap', () => {
    const diff = text(
      [
        { oldStart: 5, oldLines: 1, newStart: 5, newLines: 1, lines: [del(5, 'x'), add(5, 'y')] },
        {
          oldStart: 15,
          oldLines: 1,
          newStart: 15,
          newLines: 1,
          lines: [del(15, 'x'), add(15, 'y')],
        },
      ],
      [16, 16],
    );
    expect(picture(rows([entry('a.txt')], { 'a.txt': diff }))).toEqual([
      'header a.txt',
      'hunk @@ -5 +5 @@ [1-4] all',
      'del 5 . x',
      'add . 5 y',
      'hunk @@ -15 +15 @@ [6-14] all',
      'del 15 . x',
      'add . 15 y',
      'hunk - [16-16] all',
      'end',
    ]);
  });

  it('offers to expand up and down between hunks far apart', () => {
    const diff = text(
      [
        { oldStart: 2, oldLines: 1, newStart: 2, newLines: 1, lines: [del(2, 'x'), add(2, 'y')] },
        {
          oldStart: 80,
          oldLines: 1,
          newStart: 80,
          newLines: 1,
          lines: [del(80, 'x'), add(80, 'y')],
        },
      ],
      [80, 80],
    );
    const hunks = rows([entry('a.txt')], { 'a.txt': diff }).filter((row) => row.kind === 'hunk');
    expect(hunks.map((row) => row.directions)).toEqual([['all'], ['down', 'up']]);
  });

  it('shows revealed lines with both line numbers, and the rest stays hidden', () => {
    // One line removed at old line 10, so new line n is old line n + 1 below it.
    const diff = text(
      [{ oldStart: 10, oldLines: 1, newStart: 9, newLines: 0, lines: [del(10, 'gone')] }],
      [60, 59],
    );
    const context = new Map([
      [9, 'line 9'],
      [10, 'line 10'],
      [11, 'line 11'],
      [59, 'line 59'],
    ]);
    expect(picture(rows([entry('a.txt')], { 'a.txt': { diff, context } }))).toEqual([
      'header a.txt',
      // Line 9 is revealed, so the hidden lines above aren't next to the hunk.
      'hunk - [1-8] all',
      'ctx 9 9 line 9',
      'del 10 . gone',
      'ctx 11 10 line 10',
      'ctx 12 11 line 11',
      'hunk - [12-58] down,up',
      'ctx 60 59 line 59',
      'end',
    ]);
  });

  it('drops the hunk header once the gap above it is revealed', () => {
    const diff = text(
      [{ oldStart: 3, oldLines: 1, newStart: 3, newLines: 1, lines: [del(3, 'x'), add(3, 'y')] }],
      [3, 3],
    );
    const context = new Map([
      [1, 'one'],
      [2, 'two'],
    ]);
    expect(picture(rows([entry('a.txt')], { 'a.txt': { diff, context } }))).toEqual([
      'header a.txt',
      'ctx 1 1 one',
      'ctx 2 2 two',
      'del 3 . x',
      'add . 3 y',
      'end',
    ]);
  });

  it('marks lines with no newline at the end of the file', () => {
    const diff = text(
      [
        {
          oldStart: 1,
          oldLines: 2,
          newStart: 1,
          newLines: 2,
          lines: [ctx(1, 1, 'a'), del(2, 'b'), add(2, 'B')],
        },
      ],
      [2, 2],
      { old: true, new: false },
    );
    expect(picture(rows([entry('a.txt')], { 'a.txt': diff })).slice(2, 5)).toEqual([
      'ctx 1 1 a',
      'del 2 . b ⊘',
      'add . 2 B',
    ]);
  });
});

describe('buildRows, split', () => {
  it('aligns changed blocks with filler cells', () => {
    const diff = text(
      [
        {
          oldStart: 1,
          oldLines: 4,
          newStart: 1,
          newLines: 4,
          lines: [
            ctx(1, 1, 'same'),
            del(2, 'old a', [[4, 5]]),
            del(3, 'old b'),
            add(2, 'new a', [[4, 5]]),
            ctx(4, 3, 'same again'),
            add(4, 'added'),
          ],
        },
      ],
      [4, 4],
    );
    expect(
      picture(rows([entry('a.txt')], { 'a.txt': diff }, { style: 'split' })).slice(2, -1),
    ).toEqual([
      'ctx 1 same | ctx 1 same',
      'del 2 old a | add 2 new a',
      'del 3 old b | (filler)',
      'ctx 4 same again | ctx 3 same again',
      '(filler) | add 4 added',
    ]);
  });

  it('keeps word-level ranges on both sides', () => {
    const split = rows([entry('a.txt')], { 'a.txt': middle }, { style: 'split' }).find(
      (row) => row.kind === 'split' && row.left?.kind === 'del',
    );
    expect(split?.kind === 'split' && [split.left?.words, split.right?.words]).toEqual([
      [[0, 3]],
      [[0, 3]],
    ]);
  });

  it('shows revealed lines on both sides', () => {
    const context = new Map([[48, 'line 48']]);
    const split = rows(
      [entry('a.txt')],
      { 'a.txt': { diff: middle, context } },
      { style: 'split' },
    );
    expect(picture(split).slice(1, 4)).toEqual([
      'hunk - [1-47] up',
      'ctx 48 line 48 | ctx 48 line 48',
      'ctx 49 line 49 | ctx 49 line 49',
    ]);
  });

  it('puts the missing-newline marker on the side that lacks it', () => {
    const diff = text(
      [{ oldStart: 1, oldLines: 1, newStart: 1, newLines: 1, lines: [del(1, 'a'), add(1, 'a')] }],
      [1, 1],
      { old: false, new: true },
    );
    expect(picture(rows([entry('a.txt')], { 'a.txt': diff }, { style: 'split' }))[2]).toBe(
      'del 1 a | add 1 a ⊘',
    );
  });
});

describe('buildRows, special files', () => {
  it('shows a placeholder until the diff loads, sized from the counts', () => {
    const file = entry('a.txt', { additions: 10, deletions: 4 });
    expect(picture(rows([file], {}))).toEqual(['header a.txt', 'placeholder 21', 'end']);
    expect(estimateLines(file, 'split', 3)).toBe(17);
    expect(estimateLines(entry('b.bin', { binary: true }), 'inline', 3)).toBe(2);
  });

  it('shows only the header of a collapsed file', () => {
    expect(
      picture(rows([entry('a.txt')], { 'a.txt': middle }, { collapsed: new Set(['a.txt']) })),
    ).toEqual(['header a.txt (collapsed)', 'end (open)']);
  });

  it('shows messages for binary, large and whitespace-only files', () => {
    const files = [entry('a.bin', { binary: true }), entry('big.txt'), entry('ws.txt')];
    const result = rows(files, {
      'a.bin': { kind: 'binary', oldSize: 3, newSize: null },
      'big.txt': { kind: 'tooLarge', additions: 2000, deletions: 0 },
      'ws.txt': { kind: 'whitespaceOnly' },
    });
    expect(picture(result).filter((row) => row.startsWith('message'))).toEqual([
      'message {"kind":"binary","oldSize":3,"newSize":null}',
      'message {"kind":"tooLarge"}',
      'message {"kind":"whitespaceOnly"}',
    ]);
  });

  it('shows submodule commits without loading a diff', () => {
    const file = entry('sub', { submodule: ['0'.repeat(40), 'a'.repeat(40)] });
    expect(picture(rows([file], {}))[1]).toBe(
      `message {"kind":"submodule","from":"${'0'.repeat(40)}","to":"${'a'.repeat(40)}"}`,
    );
  });

  it('shows a mode change, above the content changes', () => {
    const modeChange: [string, string] = ['100644', '100755'];
    const onlyMode = entry('run.sh', { modeChange, additions: 0, deletions: 0 });
    const both = entry('a.txt', { modeChange });
    const result = rows([onlyMode, both], { 'run.sh': text([], [1, 1]), 'a.txt': middle });
    expect(picture(result).slice(0, 5)).toEqual([
      'header run.sh',
      'message {"kind":"modeChange","from":"100644","to":"100755"}',
      'end',
      'header a.txt',
      'message {"kind":"modeChange","from":"100644","to":"100755"}',
    ]);
  });

  it('explains a diff with no content changes', () => {
    const renamed = entry('new.txt', { status: 'R', oldPath: 'old.txt', similarity: 100 });
    const empty = entry('empty.txt', { status: 'A', additions: 0, deletions: 0 });
    const result = rows([renamed, empty], {
      'new.txt': text([], [3, 3]),
      'empty.txt': text([], [0, 0]),
    });
    expect(picture(result).filter((row) => row.startsWith('message'))).toEqual([
      'message {"kind":"noContent","text":"File renamed without changes."}',
      'message {"kind":"noContent","text":"Empty file added."}',
    ]);
  });

  it('says when a diff failed to load', () => {
    expect(picture(rows([entry('a.txt')], { 'a.txt': 'failed' }))[1]).toBe(
      'message {"kind":"failed","text":"Couldn\'t load this diff."}',
    );
  });

  it('gives every row a unique key', () => {
    const context = new Map([[48, 'line 48']]);
    for (const style of ['inline', 'split'] as const) {
      const result = rows(
        [entry('a.txt'), entry('b.txt')],
        { 'a.txt': { diff: middle, context }, 'b.txt': middle },
        { style },
      );
      expect(new Set(result.map((row) => row.key)).size).toBe(result.length);
    }
  });
});

describe('expandRange', () => {
  const gap = { start: 10, end: 60, oldOffset: 0 };

  it('reveals 20 lines next to the change, or all of them', () => {
    expect(expandRange(gap, 'up')).toEqual([41, 60]);
    expect(expandRange(gap, 'down')).toEqual([10, 29]);
    expect(expandRange(gap, 'all')).toEqual([10, 60]);
    expect(expandRange({ start: 1, end: 5, oldOffset: 0 }, 'up')).toEqual([1, 5]);
  });
});
