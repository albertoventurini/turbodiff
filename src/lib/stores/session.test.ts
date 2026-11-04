import { flushSync } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { ApiError, changeSet, emit } from '../test/fakeApi';
import type { ChangeSet, Comparison, RepoChanged, RepoOpened } from '../types';
import { defaultComparison, SessionStore } from './session.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

const opened: RepoOpened = {
  repo: { root: '/work/app', name: 'app', head: { branch: 'main', commit: 'abc' } },
  comparison: defaultComparison(),
};

const feature: Comparison = {
  base: 'refs/heads/main',
  compare: { kind: 'commit', spec: 'refs/heads/feature' },
  mode: 'threeDot',
  ignoreWhitespace: false,
};

/** A promise the test resolves or rejects later. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
}

beforeEach(() => vi.resetAllMocks());

describe('SessionStore', () => {
  it('starts with the repository from the window state', async () => {
    api.getWindowState.mockResolvedValue({ repo: opened, error: null });
    api.setComparison.mockResolvedValue(changeSet({ generation: 1 }));
    const session = new SessionStore();
    const stop = session.start();
    await vi.waitFor(() => expect(session.changes).not.toBeNull());
    expect(session.repo?.name).toBe('app');
    expect(api.setComparison).toHaveBeenCalledWith(defaultComparison());
    expect(session.generation).toBe(1);
    expect(session.loading).toBe(false);
    stop();
  });

  it('shows an error from the window state as a notice', async () => {
    const error = { kind: 'notARepository', path: '/tmp' } as const;
    api.getWindowState.mockResolvedValue({ repo: null, error });
    const session = new SessionStore();
    const stop = session.start();
    await vi.waitFor(() => expect(session.notice).toEqual(error));
    expect(session.repo).toBeNull();
    stop();
  });

  it('follows repo-opened and open-failed', async () => {
    api.getWindowState.mockResolvedValue({ repo: null, error: null });
    api.setComparison.mockResolvedValue(changeSet());
    const session = new SessionStore();
    const stop = session.start();
    await vi.waitFor(() => expect(api.getWindowState).toHaveBeenCalled());

    emit('openFailed', { error: { kind: 'notARepository', path: '/tmp' } });
    expect(session.notice?.kind).toBe('notARepository');

    emit('repoOpened', opened);
    expect(session.repo?.root).toBe('/work/app');
    // Opening a repository clears the notice.
    expect(session.notice).toBeNull();
    stop();
  });

  it('discards a response older than the latest generation', async () => {
    const session = new SessionStore();
    const first = deferred<ChangeSet>();
    const second = deferred<ChangeSet>();
    api.setComparison.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);

    const a = session.compare(defaultComparison());
    const b = session.compare(feature);
    second.resolve(changeSet({ generation: 2, totals: { files: 2, additions: 1, deletions: 1 } }));
    await b;
    first.resolve(changeSet({ generation: 1 }));
    await a;

    expect(session.generation).toBe(2);
    expect(session.changes?.generation).toBe(2);
    expect(session.comparison).toEqual(feature);
    expect(session.loading).toBe(false);
  });

  it('ignores cancelled requests and errors from replaced requests', async () => {
    const session = new SessionStore();
    const first = deferred<ChangeSet>();
    api.setComparison
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce(changeSet({ generation: 2 }));

    const a = session.compare(feature);
    const b = session.compare(defaultComparison());
    await b;
    expect(session.loading).toBe(false);
    first.reject(new ApiError({ kind: 'refNotFound', name: 'feature' }));
    await a;

    expect(session.error).toBeNull();
    expect(session.changes?.generation).toBe(2);
  });

  it('shows no error for a cancelled request', async () => {
    const session = new SessionStore();
    api.setComparison.mockRejectedValueOnce(new ApiError({ kind: 'cancelled' }));
    await session.compare(feature);
    expect(session.error).toBeNull();
  });

  it('reports an invalid ref and can go back to the default comparison', async () => {
    const session = new SessionStore();
    session.comparison = { ...feature, ignoreWhitespace: true };
    api.setComparison
      .mockRejectedValueOnce(new ApiError({ kind: 'refNotFound', name: 'refs/heads/gone' }))
      .mockResolvedValueOnce(changeSet({ generation: 2 }));

    await session.compare({ ...feature, ignoreWhitespace: true });
    expect(session.error).toEqual({ kind: 'refNotFound', name: 'refs/heads/gone' });
    expect(session.changes).toBeNull();

    await session.resetComparison();
    expect(api.setComparison).toHaveBeenLastCalledWith(defaultComparison(true));
    expect(session.error).toBeNull();
    expect(session.changes?.generation).toBe(2);
  });

  it('counts generations from zero for a newly opened repository', () => {
    const session = new SessionStore();
    api.setComparison.mockReturnValue(new Promise(() => {}));
    session.generation = 5;
    session.show(opened);
    flushSync();
    expect(session.generation).toBe(0);

    session.generation = 3;
    // The same repository with another comparison, from the command line.
    session.show({ ...opened, comparison: feature });
    expect(session.generation).toBe(3);
  });

  it('returns the value of a current response only', () => {
    const session = new SessionStore();
    expect(session.current({ generation: 2, value: 'a' })).toBe('a');
    expect(session.current({ generation: 1, value: 'b' })).toBeNull();
    expect(session.current({ generation: 2, value: 'c' })).toBe('c');
  });

  describe('live refresh', () => {
    const head = { branch: 'main', commit: 'def' };

    function refreshed(generation: number, overrides: Partial<RepoChanged> = {}): RepoChanged {
      return {
        comparison: defaultComparison(),
        changes: changeSet({ generation }),
        filesChanged: ['a.txt'],
        head,
        ...overrides,
      };
    }

    /** A session showing `opened`, listed at generation 1. */
    async function listed() {
      api.getWindowState.mockResolvedValue({ repo: opened, error: null });
      api.setComparison.mockResolvedValue(changeSet({ generation: 1 }));
      const session = new SessionStore();
      const stop = session.start();
      await vi.waitFor(() => expect(session.changes).not.toBeNull());
      return { session, stop };
    }

    it('applies repo-changed as a refresh of the list', async () => {
      const { session, stop } = await listed();
      const listing = session.listing;
      emit('repoChanged', refreshed(2));
      expect(session.changes?.generation).toBe(2);
      expect(session.generation).toBe(2);
      expect(session.refresh).toEqual({ generation: 2, filesChanged: ['a.txt'] });
      expect(session.listing).toBe(listing);
      expect(session.repo?.head).toEqual(head);
      stop();
    });

    it('ignores a refresh for an older generation or another comparison', async () => {
      const { session, stop } = await listed();
      emit('repoChanged', refreshed(3));
      emit('repoChanged', refreshed(2));
      expect(session.changes?.generation).toBe(3);
      emit('repoChanged', refreshed(4, { comparison: feature }));
      expect(session.changes?.generation).toBe(3);
      stop();
    });

    it('takes a refresh that replaced the pending listing as its result', async () => {
      const { session, stop } = await listed();
      const pending = deferred<ChangeSet>();
      api.setComparison.mockReturnValueOnce(pending.promise);
      const listing = session.listing;
      const compare = session.compare(defaultComparison());
      emit('repoChanged', refreshed(3));
      expect(session.loading).toBe(false);
      expect(session.refresh).toBeNull();
      expect(session.listing).toBe(listing + 1);
      pending.reject(new ApiError({ kind: 'cancelled' }));
      await compare;
      expect(session.changes?.generation).toBe(3);
      expect(session.error).toBeNull();
      stop();
    });

    it('falls back to work tree vs HEAD when a ref disappears', async () => {
      const { session, stop } = await listed();
      await session.compare({ ...feature, ignoreWhitespace: true });
      emit('comparisonInvalid', { name: 'refs/heads/feature' });
      expect(api.setComparison).toHaveBeenLastCalledWith(defaultComparison(true));
      expect(session.fallback).toBe('refs/heads/feature');
      // Choosing another comparison clears the notice.
      await session.compare(feature);
      expect(session.fallback).toBeNull();
      stop();
    });
  });
});
