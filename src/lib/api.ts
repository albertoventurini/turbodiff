// Typed wrappers around the generated IPC bindings in types.ts. Commands that
// can fail throw an `ApiError` carrying the backend's `AppError`.
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  commands,
  events,
  type AppError,
  type ChangeSet,
  type CommitItem,
  type Comparison,
  type ComparisonInvalid,
  type FileDiff,
  type OpenFailed,
  type RecentRepo,
  type RefItem,
  type RepoChanged,
  type RepoOpened,
  type Settings,
  type SettingsPatch,
  type Side,
  type Versioned,
  type WindowState,
} from './types';

export class ApiError extends Error {
  constructor(readonly error: AppError) {
    super(error.kind);
    this.name = 'ApiError';
  }
}

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: AppError };

async function unwrap<T>(result: Promise<Result<T>>): Promise<T> {
  const r = await result;
  if (r.status === 'error') throw new ApiError(r.error);
  return r.data;
}

export function appVersion(): Promise<string> {
  return commands.appVersion();
}

export function getWindowState(): Promise<WindowState> {
  return unwrap(commands.getWindowState());
}

export function openRepo(path: string): Promise<void> {
  return unwrap(commands.openRepo(path)).then(() => undefined);
}

export function showOpenDialog(): Promise<void> {
  return commands.showOpenDialog();
}

export function listBranches(): Promise<Versioned<RefItem[]>> {
  return unwrap(commands.listBranches());
}

export function listTags(): Promise<Versioned<RefItem[]>> {
  return unwrap(commands.listTags());
}

/** Commits from `HEAD`, newest first. */
export function listCommits(offset: number, limit: number): Promise<Versioned<CommitItem[]>> {
  return unwrap(commands.listCommits(offset, limit));
}

/** Commits whose message contains `query`, ignoring case. */
export function searchCommits(query: string, limit: number): Promise<Versioned<CommitItem[]>> {
  return unwrap(commands.searchCommits(query, limit));
}

/** The commits whose SHA starts with `prefix` (4–40 hex characters). */
export function resolveCommitPrefix(prefix: string): Promise<Versioned<CommitItem[]>> {
  return unwrap(commands.resolveCommitPrefix(prefix));
}

export function setComparison(comparison: Comparison): Promise<ChangeSet> {
  return unwrap(commands.setComparison(comparison));
}

/** The diff of one file from the current list. `force` shows a large file anyway ("Load diff"). */
export function getFileDiff(path: string, force: boolean): Promise<Versioned<FileDiff>> {
  return unwrap(commands.getFileDiff(path, force));
}

/** Lines `start..=end` (1-based, clamped to the file) of one side of a file. */
export function getLines(
  path: string,
  side: Side,
  start: number,
  end: number,
): Promise<Versioned<string[]>> {
  return unwrap(commands.getLines(path, side, start, end));
}

export function getSettings(): Promise<Settings> {
  return commands.getSettings();
}

export function setSettings(patch: SettingsPatch): Promise<Settings> {
  return commands.setSettings(patch);
}

export function listRecent(): Promise<RecentRepo[]> {
  return commands.listRecent();
}

export function removeRecent(path: string): Promise<RecentRepo[]> {
  return commands.removeRecent(path);
}

export function onRepoOpened(cb: (event: RepoOpened) => void): Promise<UnlistenFn> {
  return events.repoOpened(getCurrentWindow()).listen((e) => cb(e.payload));
}

export function onOpenFailed(cb: (event: OpenFailed) => void): Promise<UnlistenFn> {
  return events.openFailed(getCurrentWindow()).listen((e) => cb(e.payload));
}

export function onRepoChanged(cb: (event: RepoChanged) => void): Promise<UnlistenFn> {
  return events.repoChanged(getCurrentWindow()).listen((e) => cb(e.payload));
}

export function onComparisonInvalid(cb: (event: ComparisonInvalid) => void): Promise<UnlistenFn> {
  return events.comparisonInvalid(getCurrentWindow()).listen((e) => cb(e.payload));
}

export function onSettingsChanged(cb: (settings: Settings) => void): Promise<UnlistenFn> {
  return events.settingsChanged.listen((e) => cb(e.payload));
}

export function onRecentChanged(cb: (recent: RecentRepo[]) => void): Promise<UnlistenFn> {
  return events.recentChanged.listen((e) => cb(e.payload));
}

/** The OS switched between light and dark (Tauri's window theme event). */
export function onSystemThemeChanged(cb: (dark: boolean) => void): Promise<UnlistenFn> {
  return getCurrentWindow().onThemeChanged((e) => cb(e.payload === 'dark'));
}
