// The stores of one window, handed to components through Svelte context.
import { getContext, setContext } from 'svelte';
import type { RecentStore } from './recent.svelte';
import type { SessionStore } from './session.svelte';
import type { SettingsStore } from './settings.svelte';
import type { SystemMode } from '../theme/theme.svelte';
import type { ViewStore } from './view.svelte';

export type Stores = {
  session: SessionStore;
  view: ViewStore;
  settings: SettingsStore;
  recent: RecentStore;
  /** Whether the OS is in dark mode. */
  system: SystemMode;
};

const KEY = Symbol('stores');

export function setStores(stores: Stores): void {
  setContext(KEY, stores);
}

export function getStores(): Stores {
  return getContext<Stores>(KEY);
}

/** For tests, which pass the context to `render`. */
export function storesContext(stores: Stores): Map<symbol, Stores> {
  return new Map([[KEY, stores]]);
}
