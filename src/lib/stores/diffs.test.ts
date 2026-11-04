import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { ApiError, changeSet, versioned } from '../test/fakeApi';
import type { FileDiff, FileEntry } from '../types';
import { DiffStore } from './diffs.svelte';
import { SessionStore } from './session.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => vi.resetAllMocks());

function entry(path: string): FileEntry {
  return {
    path,
    oldPath: null,
    status: 'M',
    similarity: null,
    additions: 1,
    deletions: 0,
    binary: false,
    modeChange: null,
    submodule: null,
  };
}

/** Line `no` of 100 added, with no context. */
function added(no: number): FileDiff {
  return {
    kind: 'text',
    hunks: [
      {
        oldStart: no - 1,
        oldLines: 0,
        newStart: no,
        newLines: 1,
        lines: [{ kind: 'add', oldNo: null, newNo: no, text: `new ${no}`, words: null }],
      },
    ],
    oldLineCount: 99,
    newLineCount: 100,
    noNewlineAtEof: { old: false, new: false },
  };
}

const FILES = [entry('a.txt'), entry('b.txt')];
const lines = (start: number, end: number) =>
  Array.from({ length: end - start + 1 }, (_, i) => `line ${start + i}`);

/** A store with both files loaded at generation 1, and lines 1–3 of each revealed. */
async function loaded() {
  const session = new SessionStore();
  session.generation = 1;
  session.changes = changeSet({ generation: 1, files: FILES });
  const store = new DiffStore(session);
  api.getFileDiff.mockResolvedValue(versioned(added(50), 1));
  api.getLines.mockImplementation((_path, _side, start, end) =>
    Promise.resolve(versioned(lines(start, end), session.generation)),
  );
  for (const file of FILES) {
    await store.load(file.path);
    await store.expand(file.path, { start: 1, end: 49, oldOffset: 0 }, 'down');
  }
  return { session, store };
}

/** What the backend does on `repo-changed`: a new generation. */
function refresh(session: SessionStore, store: DiffStore, filesChanged: string[], files = FILES) {
  session.accept(2);
  session.changes = changeSet({ generation: 2, files });
  store.refresh({ generation: 2, filesChanged }, files);
}

describe('DiffStore.refresh', () => {
  it('keeps unchanged files, expanded context and all', async () => {
    const { session, store } = await loaded();
    const before = store.get('b.txt');
    refresh(session, store, ['a.txt']);
    expect(store.get('b.txt')).toBe(before);
    expect(store.isStale('b.txt')).toBe(false);
  });

  it('keeps showing a changed diff until it is loaded again', async () => {
    const { session, store } = await loaded();
    const before = store.get('a.txt');
    refresh(session, store, ['a.txt']);
    const stale = store.get('a.txt');
    expect(store.isStale('a.txt')).toBe(true);
    expect(stale !== 'failed' && stale?.diff).toBe(before !== 'failed' && before?.diff);

    api.getFileDiff.mockResolvedValue(versioned(added(2), 2));
    const load = store.load('a.txt');
    // Still shown while loading.
    expect(store.get('a.txt')).toBeDefined();
    await load;
    const now = store.get('a.txt');
    expect(now !== 'failed' && now?.diff).toEqual(added(2));
    expect(store.isStale('a.txt')).toBe(false);
  });

  it('reveals the same lines again where they are still hidden', async () => {
    const { session, store } = await loaded();
    refresh(session, store, ['a.txt']);
    api.getFileDiff.mockResolvedValue(versioned(added(2), 2));
    api.getLines.mockClear();
    await store.load('a.txt');
    // Line 2 is now part of the change.
    await vi.waitFor(() => expect(api.getLines).toHaveBeenCalledTimes(2));
    expect(api.getLines).toHaveBeenCalledWith('a.txt', 'new', 1, 1);
    expect(api.getLines).toHaveBeenCalledWith('a.txt', 'new', 3, 20);
    await vi.waitFor(() => {
      const now = store.get('a.txt');
      expect(now !== 'failed' && [...now!.context.keys()].length).toBe(19);
    });
  });

  it('forgets files that left the list, and older refreshes', async () => {
    const { session, store } = await loaded();
    refresh(session, store, ['b.txt'], [FILES[0]]);
    expect(store.get('b.txt')).toBeUndefined();
    const a = store.get('a.txt');
    store.refresh({ generation: 1, filesChanged: ['a.txt'] }, FILES);
    expect(store.get('a.txt')).toBe(a);
  });

  it('shows the stale diff again when its reload is out of date', async () => {
    const { session, store } = await loaded();
    refresh(session, store, ['a.txt']);
    api.getFileDiff.mockRejectedValue(new ApiError({ kind: 'cancelled' }));
    await store.load('a.txt');
    expect(store.isStale('a.txt')).toBe(true);
  });

  it("doesn't expand a stale diff", async () => {
    const { session, store } = await loaded();
    refresh(session, store, ['a.txt']);
    api.getLines.mockClear();
    await store.expand('a.txt', { start: 21, end: 49, oldOffset: 0 }, 'all');
    expect(api.getLines).not.toHaveBeenCalled();
  });

  it('shows the diffs that loadAll loads in batches, and skips loaded files', async () => {
    vi.useFakeTimers();
    try {
      const session = new SessionStore();
      session.generation = 1;
      const store = new DiffStore(session);
      api.getFileDiff.mockResolvedValue(versioned(added(50), 1));
      await store.load('a.txt');
      await store.loadAll(['a.txt', 'b.txt', 'c.txt']);
      expect(api.getFileDiff).toHaveBeenCalledTimes(3);
      expect(store.get('b.txt')).toBeUndefined();
      // A load in the meantime waits for the batch rather than asking again.
      await store.load('c.txt');
      expect(api.getFileDiff).toHaveBeenCalledTimes(3);
      vi.runAllTimers();
      expect(store.get('b.txt')).toMatchObject({ kind: 'loaded', stale: false });
      expect(store.get('c.txt')).toMatchObject({ kind: 'loaded', stale: false });
    } finally {
      vi.useRealTimers();
    }
  });
});
