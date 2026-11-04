// How the window shows the diff (TECH.md §7.1). Hide whitespace is part of
// the comparison, so it lives in the session store.
import { SvelteSet } from 'svelte/reactivity';
import type { DiffStyle } from '../types';

export type FileTreeMode = 'tree' | 'flat';

export class ViewStore {
  sidebarCollapsed = $state(false);
  /** How the sidebar lists the changed files (PRD §6.5). */
  fileTreeMode = $state<FileTreeMode>('tree');
  /** The file shown on its own (PRD §6.6), or `null` for all files. */
  selected = $state<string | null>(null);
  /**
   * The file at the top of the main pane in the all-files view, which the
   * sidebar marks as current. The diff list (M5) keeps it up to date.
   */
  current = $state<string | null>(null);
  diffStyle = $state<DiffStyle>('inline');
  /** Paths of the files collapsed in the diff list. */
  collapsed = new SvelteSet<string>();
  /** Whether the find bar is open (PRD §6.10). */
  findOpen = $state(false);
  /** Counts `Mod+F` presses, so the find bar takes focus on each one. */
  findRequests = $state(0);

  constructor(diffStyle: DiffStyle) {
    this.diffStyle = diffStyle;
  }

  /** Selects a file, or goes back to all files when it's already selected. */
  toggleSelected(path: string): void {
    this.selected = this.selected === path ? null : path;
  }

  /** Opens the find bar, or focuses it again. */
  find(): void {
    this.findOpen = true;
    this.findRequests++;
  }
}
