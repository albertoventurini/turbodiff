// Syntax highlighting for the loaded diffs (TECH.md §7.4). Each file's old and
// new contents are highlighted whole, in the worker, and kept per theme, so
// switching modes back and forth doesn't highlight again.
import { SvelteMap } from 'svelte/reactivity';
import * as api from '../api';
import { LARGE_DIFF_LINES, LARGE_FILE_BYTES, type Token } from '../diffRows';
import { Highlighter } from '../highlight/client';
import { languageFor, type LanguageId } from '../highlight/languages';
import type { Tokens } from '../highlight/tokenize';
import type { FileDiff, FileEntry, Side } from '../types';
import type { SessionStore } from './session.svelte';

type Highlighted = Record<Side, Tokens | null>;

export class HighlightStore {
  #files = new SvelteMap<string, Highlighted>();
  /**
   * The diffs files were asked for with, highlighted or not, by the same key
   * as `#files`. A refreshed file has a new diff, and is highlighted again.
   */
  #requested = new SvelteMap<string, { diff: FileDiff }>();
  #key: unknown[] = [];

  constructor(
    private session: SessionStore,
    private highlighter = new Highlighter(),
  ) {}

  /** Forgets everything when what the diffs depend on changes (see `DiffStore.sync`). */
  sync(...key: unknown[]): void {
    if (key.length === this.#key.length && key.every((value, i) => value === this.#key[i])) return;
    this.#key = key;
    this.#files.clear();
    this.#requested.clear();
  }

  /**
   * Highlights a file's loaded diff in `theme`, unless that's done or under
   * way. Files in unknown languages, large files and anything but text stay
   * plain, as does everything once the worker has failed.
   */
  request(file: FileEntry, diff: FileDiff, theme: string): void {
    const key = `${theme}\0${file.path}`;
    const previous = this.#requested.get(key);
    if (previous?.diff === diff) return;
    if (previous) {
      this.#requested.delete(key);
      this.#files.delete(key);
    }
    if (this.highlighter.broken || diff.kind !== 'text') return;
    const lang = languageFor(file.path);
    if (!lang || file.additions + file.deletions > LARGE_DIFF_LINES) return;
    const request = { diff };
    this.#requested.set(key, request);
    // Deleted lines come from the old side; added and unchanged ones from the new.
    const sides: [Side, number][] = [
      ['old', file.deletions > 0 ? diff.oldLineCount : 0],
      ['new', diff.newLineCount],
    ];
    void Promise.all(sides.map(([side, lines]) => this.#side(file.path, side, lines, lang, theme)))
      .then(([old, now]) => {
        // Unless forgotten in the meantime.
        if (this.#requested.get(key) === request && (old || now))
          this.#files.set(key, { old, new: now });
      })
      .catch(() => {});
  }

  async #side(
    path: string,
    side: Side,
    lines: number,
    lang: LanguageId,
    theme: string,
  ): Promise<Tokens | null> {
    if (lines === 0) return null;
    const text = this.session.current(await api.getLines(path, side, 1, lines))?.join('\n');
    if (text === undefined || text.length > LARGE_FILE_BYTES) return null;
    return this.highlighter.highlight(text, lang, theme);
  }

  /** The runs of line `no` (1-based) on one side of a file, if it's highlighted. */
  line(path: string, theme: string, side: Side, no: number): Token[] | undefined {
    const tokens = this.#files.get(`${theme}\0${path}`)?.[side];
    const runs = tokens?.lines[no - 1];
    return runs?.map(([end, style]) => ({
      end,
      style: style === -1 ? null : tokens!.styles[style],
    }));
  }

  /** Stops the worker. */
  stop(): void {
    this.highlighter.stop();
  }
}
