// The window's repository session: the repository, the comparison and its
// changed files (TECH.md §7.1). Responses for an older `generation` than the
// latest one seen are discarded (TECH.md §5).
import * as api from '../api';
import { ApiError } from '../api';
import type { AppError, ChangeSet, Comparison, RepoInfo, RepoOpened, Versioned } from '../types';

/** Work tree vs `HEAD`, three-dot (PRD §6.2). */
export function defaultComparison(ignoreWhitespace = false): Comparison {
  return { base: 'HEAD', compare: { kind: 'workTree' }, mode: 'threeDot', ignoreWhitespace };
}

function appError(err: unknown): AppError {
  if (err instanceof ApiError) return err.error;
  return { kind: 'internal', message: String(err) };
}

export class SessionStore {
  /** `null` while the window shows the welcome screen. */
  repo = $state<RepoInfo | null>(null);
  /** The comparison asked for last. */
  comparison = $state<Comparison>(defaultComparison());
  /** The changed files of the latest generation, once listed. */
  changes = $state<ChangeSet | null>(null);
  /** The latest generation seen. */
  generation = $state(0);
  /** Waiting for the changed files of `comparison`. */
  loading = $state(false);
  /** Why the changed files of `comparison` couldn't be listed. */
  error = $state<AppError | null>(null);
  /** A failure that leaves the window as it was, such as opening a folder that isn't a repository. */
  notice = $state<AppError | null>(null);

  #request = 0;

  /** Loads what the window shows and follows `repo-opened` and `open-failed`. */
  start(): () => void {
    const unlisteners = [
      api.onRepoOpened((event) => this.show(event)),
      api.onOpenFailed(({ error }) => (this.notice = error)),
    ];
    void Promise.all(unlisteners)
      .then(() => api.getWindowState())
      .then((state) => {
        if (state.repo) this.show(state.repo);
        if (state.error) this.notice = state.error;
      })
      .catch((err: unknown) => (this.notice = appError(err)));
    return () => unlisteners.forEach((unlisten) => void unlisten.then((stop) => stop()));
  }

  /** Shows a repository that was opened in this window. */
  show({ repo, comparison }: RepoOpened): void {
    if (this.repo?.root !== repo.root) {
      // A new backend session counts generations from zero.
      this.generation = 0;
      this.changes = null;
    }
    this.repo = repo;
    this.notice = null;
    void this.compare(comparison);
  }

  /** Switches to `comparison` and lists its changed files. */
  async compare(comparison: Comparison): Promise<void> {
    const request = ++this.#request;
    this.comparison = comparison;
    this.loading = true;
    this.error = null;
    try {
      const changes = await api.setComparison(comparison);
      if (this.accept(changes.generation)) this.changes = changes;
    } catch (err) {
      const error = appError(err);
      // A newer request replaced this one.
      if (request !== this.#request || error.kind === 'cancelled') return;
      this.error = error;
      this.changes = null;
    } finally {
      if (request === this.#request) this.loading = false;
    }
  }

  /** Goes back to work tree vs `HEAD`, keeping hide whitespace as it is. */
  resetComparison(): Promise<void> {
    return this.compare(defaultComparison(this.comparison.ignoreWhitespace));
  }

  /**
   * Whether a response computed for `generation` is still current. A newer
   * one becomes the latest generation.
   */
  accept(generation: number): boolean {
    if (generation < this.generation) return false;
    this.generation = generation;
    return true;
  }

  /** The value of a response, or `null` when a newer generation replaced it. */
  current<T>(response: Versioned<T>): T | null {
    return this.accept(response.generation) ? response.value : null;
  }
}
