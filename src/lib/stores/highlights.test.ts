import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LARGE_FILE_BYTES } from '../diffRows';
import type { Highlighter } from '../highlight/client';
import type { Tokens } from '../highlight/tokenize';
import * as api from '../test/fakeApi';
import { versioned } from '../test/fakeApi';
import type { FileDiff, FileEntry, Side } from '../types';
import { HighlightStore } from './highlights.svelte';
import { SessionStore } from './session.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => vi.resetAllMocks());

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

const diff: FileDiff = {
  kind: 'text',
  hunks: [],
  oldLineCount: 2,
  newLineCount: 3,
  noNewlineAtEof: { old: false, new: false },
};

const FILES: Record<Side, string[]> = { old: ['let a;', 'b'], new: ['let a;', 'c', 'd'] };

/** Each line one run: the whole line, styled with its side, text and the theme. */
function fakeTokens(text: string, theme: string): Tokens {
  const lines = text.split('\n');
  return {
    styles: lines.map((line) => `${theme}:${line}`),
    lines: lines.map((line, i) => [[line.length, i]]),
  };
}

function setup(generation = 1) {
  const session = new SessionStore();
  session.generation = generation;
  const highlight = vi.fn((text: string, _lang: string, theme: string) =>
    Promise.resolve<Tokens | null>(fakeTokens(text, theme)),
  );
  const highlighter = { broken: false, highlight, stop: vi.fn() };
  api.getLines.mockImplementation((_path, side, start, end) =>
    Promise.resolve(versioned(FILES[side].slice(start - 1, end), 1)),
  );
  const store = new HighlightStore(session, highlighter as unknown as Highlighter);
  store.sync('changes');
  return { session, store, highlight, highlighter };
}

describe('HighlightStore', () => {
  it('highlights both sides of a file whole', async () => {
    const { store, highlight } = setup();
    store.request(entry('a.ts'), diff, 'github-light');
    await vi.waitFor(() => expect(store.line('a.ts', 'github-light', 'new', 3)).toBeDefined());
    expect(api.getLines).toHaveBeenCalledWith('a.ts', 'old', 1, 2);
    expect(api.getLines).toHaveBeenCalledWith('a.ts', 'new', 1, 3);
    expect(highlight).toHaveBeenCalledWith('let a;\nb', 'typescript', 'github-light');
    expect(store.line('a.ts', 'github-light', 'old', 2)).toEqual([
      { end: 1, style: 'github-light:b' },
    ]);
    expect(store.line('a.ts', 'github-light', 'new', 3)).toEqual([
      { end: 1, style: 'github-light:d' },
    ]);
    // Not in the other theme yet.
    expect(store.line('a.ts', 'github-dark', 'new', 3)).toBeUndefined();
  });

  it("skips the old side when nothing's deleted", async () => {
    const { store } = setup();
    store.request(entry('a.ts', { deletions: 0 }), diff, 'github-light');
    await vi.waitFor(() => expect(store.line('a.ts', 'github-light', 'new', 1)).toBeDefined());
    expect(api.getLines).toHaveBeenCalledTimes(1);
    expect(store.line('a.ts', 'github-light', 'old', 1)).toBeUndefined();
  });

  it('highlights each file once per theme', async () => {
    const { store, highlight } = setup();
    store.request(entry('a.ts'), diff, 'github-light');
    store.request(entry('a.ts'), diff, 'github-light');
    store.request(entry('a.ts'), diff, 'github-dark');
    await vi.waitFor(() => expect(store.line('a.ts', 'github-dark', 'new', 1)).toBeDefined());
    expect(highlight).toHaveBeenCalledTimes(4);
    expect(store.line('a.ts', 'github-light', 'new', 1)).toBeDefined();
  });

  it('leaves unknown languages, large files and special diffs plain', async () => {
    const { store, highlighter } = setup();
    store.request(entry('notes.txt'), diff, 'github-light');
    store.request(entry('big.ts', { additions: 1001, deletions: 0 }), diff, 'github-light');
    store.request(entry('bin.ts'), { kind: 'binary', oldSize: 1, newSize: 2 }, 'github-light');
    highlighter.broken = true;
    store.request(entry('a.ts'), diff, 'github-light');
    await Promise.resolve();
    expect(api.getLines).not.toHaveBeenCalled();
  });

  it('leaves a side plain when its text is too long', async () => {
    const { store, highlight } = setup();
    api.getLines.mockImplementation((_path, side) =>
      Promise.resolve(versioned(side === 'old' ? ['x'.repeat(LARGE_FILE_BYTES + 1)] : ['y'], 1)),
    );
    store.request(entry('a.ts'), diff, 'github-light');
    await vi.waitFor(() => expect(store.line('a.ts', 'github-light', 'new', 1)).toBeDefined());
    expect(highlight).toHaveBeenCalledTimes(1);
    expect(store.line('a.ts', 'github-light', 'old', 1)).toBeUndefined();
  });

  it('drops text from an older generation', async () => {
    const { store, highlight } = setup(2);
    store.request(entry('a.ts'), diff, 'github-light');
    await vi.waitFor(() => expect(api.getLines).toHaveBeenCalledTimes(2));
    await new Promise((resolve) => setTimeout(resolve));
    expect(highlight).not.toHaveBeenCalled();
    expect(store.line('a.ts', 'github-light', 'new', 1)).toBeUndefined();
  });

  it('stays plain when highlighting fails', async () => {
    const { store, highlight } = setup();
    highlight.mockResolvedValue(null);
    store.request(entry('a.ts'), diff, 'github-light');
    await new Promise((resolve) => setTimeout(resolve));
    expect(highlight).toHaveBeenCalled();
    expect(store.line('a.ts', 'github-light', 'new', 1)).toBeUndefined();
  });

  it('forgets everything when the diffs change, even highlighting under way', async () => {
    const { store, highlight } = setup();
    store.request(entry('a.ts'), diff, 'github-light');
    await vi.waitFor(() => expect(store.line('a.ts', 'github-light', 'new', 1)).toBeDefined());

    store.sync('other changes');
    expect(store.line('a.ts', 'github-light', 'new', 1)).toBeUndefined();
    const finish: (() => void)[] = [];
    highlight.mockImplementation(
      (text, _lang, theme) =>
        new Promise((resolve) => finish.push(() => resolve(fakeTokens(text, theme)))),
    );
    store.request(entry('b.ts'), diff, 'github-light');
    await vi.waitFor(() => expect(highlight).toHaveBeenCalledTimes(4));
    store.sync('newer changes');
    finish.forEach((done) => done());
    await new Promise((resolve) => setTimeout(resolve));
    expect(store.line('b.ts', 'github-light', 'new', 1)).toBeUndefined();
  });
});
