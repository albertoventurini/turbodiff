// Themes and light/dark mode (TECH.md §6). The window's boot script sets the
// first mode before the page paints; this keeps it in step afterwards.
import { onSystemThemeChanged } from '../api';
import type { Appearance, Theme } from '../types';

export type ColorMode = 'light' | 'dark';

/** The themes that ship, each a token set in themes.css plus its Shiki themes. */
export const THEMES: Record<Theme, { shiki: Record<ColorMode, string> }> = {
  github: { shiki: { light: 'github-light', dark: 'github-dark' } },
};

export function resolveMode(appearance: Appearance, systemDark: boolean): ColorMode {
  if (appearance === 'system') return systemDark ? 'dark' : 'light';
  return appearance;
}

export function applyTheme(root: HTMLElement, theme: Theme, mode: ColorMode): void {
  root.dataset.theme = theme;
  root.dataset.mode = mode;
}

const DARK_QUERY = '(prefers-color-scheme: dark)';

/** Whether the OS is in dark mode, following changes live. */
export class SystemMode {
  dark = $state(window.matchMedia(DARK_QUERY).matches);

  /**
   * Listens to both `prefers-color-scheme` and Tauri's theme event, since
   * either may be the first (or only) one to notice a change. Returns a
   * function that stops listening.
   */
  start(): () => void {
    const query = window.matchMedia(DARK_QUERY);
    const onQuery = (e: MediaQueryListEvent) => (this.dark = e.matches);
    query.addEventListener('change', onQuery);
    const unlisten = onSystemThemeChanged((dark) => (this.dark = dark));
    return () => {
      query.removeEventListener('change', onQuery);
      void unlisten.then((stop) => stop());
    };
  }
}
