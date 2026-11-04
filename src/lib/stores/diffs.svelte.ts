// The diffs of the files in the diff list, loaded as they near the viewport
// (TECH.md §7.3), with the unchanged lines revealed by expanding context.
import { SvelteMap } from 'svelte/reactivity';
import * as api from '../api';
import { ApiError } from '../api';
import {
  expandRange,
  hiddenRuns,
  NO_CONTEXT,
  revealLines,
  type ExpandDirection,
  type Gap,
  type LoadedDiff,
} from '../diffRows';
import type { FileEntry } from '../types';
import type { SessionStore, Refresh } from './session.svelte';

/**
 * A loaded diff. `stale` after a refresh changed the file, until it's loaded
 * again; `forced` when it was loaded with "Load diff".
 */
export type Shown = LoadedDiff & { stale: boolean; forced: boolean };

type State =
  { kind: 'loading'; shown: Shown | null } | ({ kind: 'loaded' } & Shown) | { kind: 'failed' };

/** Errors that only mean the request is out of date. */
function stale(err: unknown): boolean {
  return err instanceof ApiError && ['cancelled', 'fileNotFound'].includes(err.error.kind);
}

export class DiffStore {
  #states = new SvelteMap<string, State>();
  #key: unknown[] = [];
  #refreshed = 0;

  constructor(private session: SessionStore) {}

  /**
   * Forgets every diff when anything they depend on changes: the changed
   * files, or the settings that shape a diff.
   */
  sync(...key: unknown[]): void {
    if (key.length === this.#key.length && key.every((value, i) => value === this.#key[i])) return;
    this.#key = key;
    this.#states.clear();
  }

  /**
   * Applies a refresh of the file list (PRD §6.9). Files that left the list
   * are forgotten. Changed files keep showing their diff, marked stale, until
   * they're loaded again; the rest stay as they are, expanded context and all.
   */
  refresh({ generation, filesChanged }: Refresh, files: readonly FileEntry[]): void {
    if (generation <= this.#refreshed) return;
    this.#refreshed = generation;
    // A local lookup, never stored, so it needn't be reactive.
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const present = new Set(files.map((file) => file.path));
    for (const path of [...this.#states.keys()]) {
      if (!present.has(path)) this.#states.delete(path);
    }
    for (const path of filesChanged) {
      const state = this.#states.get(path);
      const shown =
        state?.kind === 'loaded' ? state : state?.kind === 'loading' ? state.shown : null;
      if (shown) this.#states.set(path, { ...shown, kind: 'loaded', stale: true });
      else this.#states.delete(path);
    }
  }

  /** The diff to show, `'failed'`, or `undefined` while there's none. */
  get(path: string): Shown | 'failed' | undefined {
    const state = this.#states.get(path);
    if (state?.kind === 'loaded') return state;
    if (state?.kind === 'loading') return state.shown ?? undefined;
    return state?.kind === 'failed' ? 'failed' : undefined;
  }

  /** Whether a file shows a stale diff that isn't being loaded again yet. */
  isStale(path: string): boolean {
    const state = this.#states.get(path);
    return state?.kind === 'loaded' && state.stale;
  }

  /**
   * Loads a file's diff, unless it's loaded or loading. A stale diff is loaded
   * again, and keeps showing meanwhile. `force` loads a large file anyway.
   */
  async load(path: string, force = false): Promise<void> {
    const previous = this.#states.get(path);
    const reload = previous?.kind === 'loaded' && (force || previous.stale);
    if (previous && !reload) return;
    const shown = reload ? previous : null;
    force ||= shown?.forced ?? false;
    const state: State = { kind: 'loading', shown };
    this.#states.set(path, state);
    const current = () => this.#states.get(path) === state;
    // A newer generation replaced the request: show what was there, still
    // stale, so it's loaded again.
    const restore = () => {
      if (shown) this.#states.set(path, { ...shown, kind: 'loaded' });
      else this.#states.delete(path);
    };
    try {
      const diff = this.session.current(await api.getFileDiff(path, force));
      if (!current()) return;
      if (!diff) return restore();
      this.#states.set(path, {
        kind: 'loaded',
        diff,
        context: NO_CONTEXT,
        stale: false,
        forced: force,
      });
      if (shown && shown.context.size > 0) void this.#reveal(path, shown.context.keys());
    } catch (err) {
      if (!current()) return;
      if (stale(err)) restore();
      else this.#states.set(path, { kind: 'failed' });
    }
  }

  /** Reveals again the unchanged lines that were expanded before a refresh. */
  async #reveal(path: string, lines: Iterable<number>): Promise<void> {
    const state = this.#states.get(path);
    if (state?.kind !== 'loaded') return;
    for (const [start, end] of hiddenRuns(lines, state.diff)) {
      await this.#revealRange(path, state.diff, start, end);
    }
  }

  async #revealRange(
    path: string,
    diff: LoadedDiff['diff'],
    start: number,
    end: number,
  ): Promise<void> {
    let lines: string[] | null;
    try {
      lines = this.session.current(await api.getLines(path, 'new', start, end));
    } catch (err) {
      if (stale(err)) return;
      throw err;
    }
    const now = this.#states.get(path);
    if (!lines || now?.kind !== 'loaded' || now.diff !== diff) return;
    this.#states.set(path, { ...now, context: revealLines(now.context, start, lines) });
  }

  /**
   * Reveals hidden lines of `gap` towards `direction`. Not while the diff is
   * stale, since the file on disk no longer matches it.
   */
  async expand(path: string, gap: Gap, direction: ExpandDirection): Promise<void> {
    const state = this.#states.get(path);
    if (state?.kind !== 'loaded' || state.stale || state.diff.kind !== 'text') return;
    const [start, end] = expandRange(gap, direction);
    // Unchanged lines are the same on both sides, and a file with no new
    // side has none.
    await this.#revealRange(path, state.diff, start, end);
  }
}
