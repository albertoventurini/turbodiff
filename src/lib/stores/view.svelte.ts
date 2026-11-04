// How the window shows the diff (TECH.md §7.1). Hide whitespace is part of
// the comparison, so it lives in the session store.
import { SvelteSet } from 'svelte/reactivity';
import type { DiffStyle } from '../types';

export class ViewStore {
  sidebarCollapsed = $state(false);
  /** The file shown on its own (PRD §6.6), or `null` for all files. */
  selected = $state<string | null>(null);
  diffStyle = $state<DiffStyle>('inline');
  /** Paths of the files collapsed in the diff list. */
  collapsed = new SvelteSet<string>();

  constructor(diffStyle: DiffStyle) {
    this.diffStyle = diffStyle;
  }
}
