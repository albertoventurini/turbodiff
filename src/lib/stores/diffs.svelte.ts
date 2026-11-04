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

/**
 * How long loads for find in page wait to be shown together. Each change to
 * the loaded diffs rebuilds the whole row list, so thousands of them one by
 * one would keep the window busy (TECH.md §7.5).
 */
const BATCH_MS = 250;

/** Errors that only mean the request is out of date. */
function stale(err: unknown): boolean {
  return err instanceof ApiError && ['cancelled', 'fileNotFound'].includes(err.error.kind);
}

export class DiffStore {
  #states = new SvelteMap<string, State>();
  /**
   * Changes from batched loads that aren't shown yet; `null` deletes. Not
   * reactive on purpose: nothing shows them until `#flush`.
   */
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  #pending = new Map<string, State | null>();
  #timer: ReturnType<typeof setTimeout> | null = null;
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
    this.#pending.clear();
  }

  /** A file's state, including a batched change that isn't shown yet. */
  #peek(path: string): State | undefined {
    return this.#pending.has(path)
      ? (this.#pending.get(path) ?? undefined)
      : this.#states.get(path);
  }

  /** Sets (or with `null`, deletes) a file's state, now or with the next batch. */
  #write(path: string, state: State | null, batched: boolean): void {
    if (batched) {
      this.#pending.set(path, state);
      this.#timer ??= setTimeout(() => this.#flush(), BATCH_MS);
      return;
    }
    this.#pending.delete(path);
    if (state) this.#states.set(path, state);
    else this.#states.delete(path);
  }

  /** Shows the batched changes, all at once. */
  #flush(): void {
    if (this.#timer !== null) clearTimeout(this.#timer);
    this.#timer = null;
    for (const [path, state] of this.#pending) {
      if (state) this.#states.set(path, state);
      else this.#states.delete(path);
    }
    this.#pending.clear();
  }

  /**
   * Applies a refresh of the file list (PRD §6.9). Files that left the list
   * are forgotten. Changed files keep showing their diff, marked stale, until
   * they're loaded again; the rest stay as they are, expanded context and all.
   */
  refresh({ generation, filesChanged }: Refresh, files: readonly FileEntry[]): void {
    if (generation <= this.#refreshed) return;
    this.#refreshed = generation;
    this.#flush();
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
   * A `batched` load is shown with the next batch.
   */
  async load(path: string, force = false, batched = false): Promise<void> {
    const previous = this.#peek(path);
    const reload = previous?.kind === 'loaded' && (force || previous.stale);
    if (previous && !reload) return;
    const shown = reload ? previous : null;
    force ||= shown?.forced ?? false;
    const state: State = { kind: 'loading', shown };
    this.#write(path, state, batched);
    const current = () => this.#peek(path) === state;
    // A newer generation replaced the request: show what was there, still
    // stale, so it's loaded again.
    const restore = () => this.#write(path, shown && { ...shown, kind: 'loaded' }, batched);
    try {
      const diff = this.session.current(await api.getFileDiff(path, force));
      if (!current()) return;
      if (!diff) return restore();
      this.#write(
        path,
        { kind: 'loaded', diff, context: NO_CONTEXT, stale: false, forced: force },
        batched,
      );
      if (shown && shown.context.size > 0) void this.#reveal(path, shown.context.keys());
    } catch (err) {
      if (!current()) return;
      if (stale(err)) restore();
      else this.#write(path, { kind: 'failed' }, batched);
    }
  }

  /**
   * Loads the diffs of every file in `paths` that has none yet, a few at a
   * time and shown in batches, for find in page.
   */
  async loadAll(paths: readonly string[], concurrency = 4): Promise<void> {
    const queue = paths.filter((path) => this.#peek(path) === undefined);
    const worker = async () => {
      for (let path = queue.shift(); path !== undefined; path = queue.shift()) {
        await this.load(path, false, true);
      }
    };
    await Promise.all(Array.from({ length: Math.min(concurrency, queue.length) }, worker));
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
