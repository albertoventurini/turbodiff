// The recent repositories list on the welcome screen (PRD §6.1).
import * as api from '../api';
import type { RecentRepo } from '../types';

export class RecentStore {
  items = $state<RecentRepo[]>([]);

  /** Loads the list and follows changes. Returns a function that stops. */
  start(): () => void {
    let changed = false;
    const unlisten = api.onRecentChanged((items) => {
      changed = true;
      this.items = items;
    });
    void api.listRecent().then((items) => {
      // An event that arrived meanwhile is newer.
      if (!changed) this.items = items;
    });
    return () => void unlisten.then((stop) => stop());
  }

  async remove(path: string): Promise<void> {
    this.items = await api.removeRecent(path);
  }
}
