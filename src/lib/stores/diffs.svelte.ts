// The diffs of the files in the diff list, loaded as they near the viewport
// (TECH.md §7.3), with the unchanged lines revealed by expanding context.
import { SvelteMap } from 'svelte/reactivity';
import * as api from '../api';
import { ApiError } from '../api';
import {
  expandRange,
  NO_CONTEXT,
  revealLines,
  type ExpandDirection,
  type Gap,
  type LoadedDiff,
} from '../diffRows';
import type { SessionStore } from './session.svelte';

type State = { kind: 'loading' } | ({ kind: 'loaded' } & LoadedDiff) | { kind: 'failed' };

/** Errors that only mean the request is out of date. */
function stale(err: unknown): boolean {
  return err instanceof ApiError && ['cancelled', 'fileNotFound'].includes(err.error.kind);
}

export class DiffStore {
  #states = new SvelteMap<string, State>();
  #key: unknown[] = [];

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

  /** The loaded diff, `'failed'`, or `undefined` while it isn't loaded. */
  get(path: string): LoadedDiff | 'failed' | undefined {
    const state = this.#states.get(path);
    if (state?.kind === 'loaded') return state;
    return state?.kind === 'failed' ? 'failed' : undefined;
  }

  /** Loads a file's diff, unless it's loaded or loading. `force` loads a large file anyway. */
  async load(path: string, force = false): Promise<void> {
    const previous = this.#states.get(path);
    if (previous && !(force && previous.kind === 'loaded')) return;
    const state: State = { kind: 'loading' };
    this.#states.set(path, state);
    const current = () => this.#states.get(path) === state;
    try {
      const diff = this.session.current(await api.getFileDiff(path, force));
      if (!current()) return;
      if (diff) this.#states.set(path, { kind: 'loaded', diff, context: NO_CONTEXT });
      else this.#states.delete(path);
    } catch (err) {
      if (!current()) return;
      if (stale(err)) this.#states.delete(path);
      else this.#states.set(path, { kind: 'failed' });
    }
  }

  /** Reveals hidden lines of `gap` towards `direction`. */
  async expand(path: string, gap: Gap, direction: ExpandDirection): Promise<void> {
    const state = this.#states.get(path);
    if (state?.kind !== 'loaded' || state.diff.kind !== 'text') return;
    const [start, end] = expandRange(gap, direction);
    // Unchanged lines are the same on both sides, and a file with no new
    // side has none.
    let lines: string[] | null;
    try {
      lines = this.session.current(await api.getLines(path, 'new', start, end));
    } catch (err) {
      if (stale(err)) return;
      throw err;
    }
    const now = this.#states.get(path);
    if (!lines || now?.kind !== 'loaded' || now.diff !== state.diff) return;
    this.#states.set(path, { ...now, context: revealLines(now.context, start, lines) });
  }
}
