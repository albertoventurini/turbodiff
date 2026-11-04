// A stand-in for api.ts in component tests: `vi.mock('../api', () => import('../test/fakeApi'))`.
// Commands are `vi.fn`s with harmless defaults, and `emit` fires events.
import { vi } from 'vitest';
import type {
  AppError,
  ChangeSet,
  CommitItem,
  ComparisonInvalid,
  FileDiff,
  OpenFailed,
  RecentRepo,
  RefItem,
  RepoChanged,
  RepoOpened,
  Settings,
  Side,
  Versioned,
  WindowState,
} from '../types';
import { DEFAULT_SETTINGS } from '../stores/settings.svelte';

export class ApiError extends Error {
  constructor(readonly error: AppError) {
    super(error.kind);
  }
}

type Listeners = {
  repoOpened: ((e: RepoOpened) => void)[];
  openFailed: ((e: OpenFailed) => void)[];
  repoChanged: ((e: RepoChanged) => void)[];
  comparisonInvalid: ((e: ComparisonInvalid) => void)[];
  settingsChanged: ((s: Settings) => void)[];
  recentChanged: ((r: RecentRepo[]) => void)[];
  systemTheme: ((dark: boolean) => void)[];
};

const listeners: Listeners = {
  repoOpened: [],
  openFailed: [],
  repoChanged: [],
  comparisonInvalid: [],
  settingsChanged: [],
  recentChanged: [],
  systemTheme: [],
};

function listen<K extends keyof Listeners>(key: K) {
  return (cb: Listeners[K][number]) => {
    (listeners[key] as unknown[]).push(cb);
    return Promise.resolve(() => {
      const list = listeners[key] as unknown[];
      list.splice(list.indexOf(cb), 1);
    });
  };
}

/** Fires an event at every listener. */
export function emit<K extends keyof Listeners>(
  key: K,
  payload: Parameters<Listeners[K][number]>[0],
) {
  for (const cb of [...listeners[key]]) (cb as (p: typeof payload) => void)(payload);
}

export function changeSet(overrides: Partial<ChangeSet> = {}): ChangeSet {
  return {
    generation: 1,
    files: [],
    totals: { files: 0, additions: 0, deletions: 0 },
    noMergeBase: false,
    ...overrides,
  };
}

/** A response for generation 0, which the session store always accepts. */
export function versioned<T>(value: T, generation = 0): Versioned<T> {
  return { generation, value };
}

export const appVersion = vi.fn(() => Promise.resolve('0.0.0'));
export const getWindowState = vi.fn((): Promise<WindowState> =>
  Promise.resolve({ repo: null, error: null }),
);
export const openRepo = vi.fn<(path: string) => Promise<void>>(() => Promise.resolve());
export const showOpenDialog = vi.fn(() => Promise.resolve());
export const listBranches = vi.fn(() => Promise.resolve(versioned<RefItem[]>([])));
export const listTags = vi.fn(() => Promise.resolve(versioned<RefItem[]>([])));
export const listCommits = vi.fn<
  (offset: number, limit: number) => Promise<Versioned<CommitItem[]>>
>(() => Promise.resolve(versioned([])));
export const searchCommits = vi.fn<
  (query: string, limit: number) => Promise<Versioned<CommitItem[]>>
>(() => Promise.resolve(versioned([])));
export const resolveCommitPrefix = vi.fn<(prefix: string) => Promise<Versioned<CommitItem[]>>>(() =>
  Promise.resolve(versioned([])),
);
export const setComparison = vi.fn(() => Promise.resolve(changeSet()));
export const getFileDiff = vi.fn<(path: string, force: boolean) => Promise<Versioned<FileDiff>>>(
  () => Promise.resolve(versioned<FileDiff>({ kind: 'whitespaceOnly' })),
);
export const getLines = vi.fn<
  (path: string, side: Side, start: number, end: number) => Promise<Versioned<string[]>>
>(() => Promise.resolve(versioned([])));
export const getSettings = vi.fn(() => Promise.resolve(DEFAULT_SETTINGS));
export const setSettings = vi.fn((patch: Partial<Settings>) =>
  Promise.resolve({ ...DEFAULT_SETTINGS, ...patch }),
);
export const resetSettings = vi.fn(() =>
  Promise.resolve({ ...DEFAULT_SETTINGS, sidebarWidth: 320 }),
);
export const listMonospaceFonts = vi.fn((): Promise<string[]> => Promise.resolve([]));
export const listRecent = vi.fn((): Promise<RecentRepo[]> => Promise.resolve([]));
export const removeRecent = vi.fn<(path: string) => Promise<RecentRepo[]>>(() =>
  Promise.resolve([]),
);

export const onRepoOpened = listen('repoOpened');
export const onOpenFailed = listen('openFailed');
export const onRepoChanged = listen('repoChanged');
export const onComparisonInvalid = listen('comparisonInvalid');
export const onSettingsChanged = listen('settingsChanged');
export const onRecentChanged = listen('recentChanged');
export const onSystemThemeChanged = listen('systemTheme');
