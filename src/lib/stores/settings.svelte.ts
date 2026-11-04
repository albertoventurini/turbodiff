// The app settings, shared by every window through `settings-changed`
// (TECH.md §7.1).
import * as api from '../api';
import type { Settings, SettingsPatch } from '../types';

declare global {
  interface Window {
    /** Set by the window's boot script (src-tauri/src/app/boot.js). */
    __TURBODIFF_SETTINGS__?: Settings;
  }
}

/** The defaults from src-tauri/src/app/settings.rs, for a page without a boot script. */
export const DEFAULT_SETTINGS: Settings = {
  appearance: 'system',
  theme: 'github',
  diffStyle: 'inline',
  codeFont: '',
  fontSize: 13,
  tabWidth: 4,
  lineWrapping: false,
  contextLines: 3,
  diffAlgorithm: 'histogram',
  sidebarWidth: 280,
};

/** The ranges the backend clamps numbers to (src-tauri/src/app/settings.rs). */
export const SIDEBAR_WIDTHS = { min: 160, max: 800 };
export const FONT_SIZES = { min: 10, max: 24 };
export const CONTEXT_LINES = { min: 0, max: 20 };

export class SettingsStore {
  /** Starts from the boot script's copy, so the first render is already right. */
  value = $state<Settings>(window.__TURBODIFF_SETTINGS__ ?? DEFAULT_SETTINGS);

  /** Loads the settings and follows changes. Returns a function that stops. */
  start(): () => void {
    let changed = false;
    const unlisten = api.onSettingsChanged((settings) => {
      changed = true;
      this.value = settings;
    });
    void unlisten.then(() =>
      api.getSettings().then((settings) => {
        // An event that arrived meanwhile is newer.
        if (!changed) this.value = settings;
      }),
    );
    return () => void unlisten.then((stop) => stop());
  }

  async update(patch: SettingsPatch): Promise<void> {
    this.value = await api.setSettings(patch);
  }

  /** Restores the defaults of every setting in the Settings window. */
  async reset(): Promise<void> {
    this.value = await api.resetSettings();
  }
}
