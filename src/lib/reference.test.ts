import { describe, expect, it } from 'vitest';
import { codeReference, comparisonLabel, type SelectedLine } from './reference';
import type { Comparison, FileEntry } from './types';

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

const workTree: Comparison = {
  base: 'HEAD',
  compare: { kind: 'workTree' },
  mode: 'threeDot',
  ignoreWhitespace: false,
};

const ctx = (no: number, text: string): SelectedLine => ({
  kind: 'ctx',
  oldNo: no - 2,
  newNo: no,
  text,
});
const add = (no: number, text: string): SelectedLine => ({
  kind: 'add',
  oldNo: null,
  newNo: no,
  text,
});
const del = (no: number, text: string): SelectedLine => ({
  kind: 'del',
  oldNo: no,
  newNo: null,
  text,
});

describe('comparisonLabel', () => {
  it('names the work tree, three-dot and two-dot comparisons', () => {
    expect(comparisonLabel(workTree)).toBe('HEAD → working tree');
    const refs: Comparison = {
      ...workTree,
      base: 'refs/heads/main',
      compare: { kind: 'commit', spec: 'refs/heads/feature' },
    };
    expect(comparisonLabel(refs)).toBe('main...feature');
    expect(comparisonLabel({ ...refs, mode: 'twoDot' })).toBe('main..feature');
    // Without a merge base, three-dot falls back to two-dot.
    expect(comparisonLabel(refs, true)).toBe('main..feature');
    const sha = 'a'.repeat(40);
    expect(comparisonLabel({ ...refs, compare: { kind: 'commit', spec: sha } })).toBe(
      'main...aaaaaaa',
    );
  });
});

describe('codeReference', () => {
  it('copies unchanged lines as plain code in the file’s language', () => {
    const text = codeReference(
      [{ file: entry('src/main.rs'), lines: [ctx(42, 'fn main() {'), ctx(43, '}')] }],
      workTree,
    );
    expect(text).toBe('src/main.rs:42-43, HEAD → working tree:\n```rust\nfn main() {\n}\n```\n');
  });

  it('copies changed lines as a diff, with the old lines of deleted ones', () => {
    const text = codeReference(
      [{ file: entry('a.ts'), lines: [ctx(10, 'a'), del(9, 'old'), add(11, 'new')] }],
      workTree,
    );
    expect(text).toBe(
      'a.ts:10-11 (old version: a.ts:9), HEAD → working tree:\n```diff\n a\n-old\n+new\n```\n',
    );
  });

  it('names the old path for deleted lines of a renamed file', () => {
    const file = entry('new.txt', { oldPath: 'old.txt', status: 'R' });
    expect(codeReference([{ file, lines: [del(3, 'x'), del(4, 'y')] }], workTree)).toBe(
      'old.txt:3-4 (old version), HEAD → working tree:\n```diff\n-x\n-y\n```\n',
    );
  });

  it('copies a part of a line as it is, naming the line', () => {
    expect(codeReference([{ file: entry('notes'), lines: [add(7, 'part')] }], workTree)).toBe(
      'notes:7, HEAD → working tree:\n```diff\n+part\n```\n',
    );
    expect(codeReference([{ file: entry('notes'), lines: [ctx(7, 'part')] }], workTree)).toBe(
      'notes:7, HEAD → working tree:\n```\npart\n```\n',
    );
  });

  it('uses a longer fence for code with backticks, and a block per file', () => {
    const text = codeReference(
      [
        { file: entry('a.md'), lines: [ctx(1, '```js')] },
        { file: entry('b.txt'), lines: [] },
        { file: entry('c.txt'), lines: [add(2, 'c')] },
      ],
      workTree,
    );
    expect(text).toBe(
      'a.md:1, HEAD → working tree:\n````markdown\n```js\n````\n\n' +
        'c.txt:2, HEAD → working tree:\n```diff\n+c\n```\n',
    );
  });

  it('copies nothing when no lines are selected', () => {
    expect(codeReference([{ file: entry('a.txt'), lines: [] }], workTree)).toBe('');
  });
});
