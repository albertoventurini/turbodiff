import { afterEach, describe, expect, it, vi } from 'vitest';
import { emit } from '../test/fakeApi';
import { applyTheme, resolveMode, SystemMode } from './theme.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

/** A matchMedia whose dark-mode answer the test can change. */
function fakeMatchMedia(dark: boolean) {
  const listeners = new Set<(e: { matches: boolean }) => void>();
  vi.stubGlobal('matchMedia', (media: string) => ({
    media,
    get matches() {
      return dark;
    },
    addEventListener: (_: string, cb: (e: { matches: boolean }) => void) => listeners.add(cb),
    removeEventListener: (_: string, cb: (e: { matches: boolean }) => void) => listeners.delete(cb),
  }));
  return {
    set(value: boolean) {
      dark = value;
      listeners.forEach((cb) => cb({ matches: value }));
    },
    listeners,
  };
}

afterEach(() => vi.unstubAllGlobals());

describe('resolveMode', () => {
  it('follows the OS only for System', () => {
    expect(resolveMode('system', true)).toBe('dark');
    expect(resolveMode('system', false)).toBe('light');
    expect(resolveMode('light', true)).toBe('light');
    expect(resolveMode('dark', false)).toBe('dark');
  });
});

describe('applyTheme', () => {
  it('sets the theme and mode on the element', () => {
    const root = document.createElement('html');
    applyTheme(root, 'github', 'dark');
    expect(root.dataset.theme).toBe('github');
    expect(root.dataset.mode).toBe('dark');
  });
});

describe('SystemMode', () => {
  it('starts from prefers-color-scheme and follows its changes', () => {
    const media = fakeMatchMedia(true);
    const system = new SystemMode();
    expect(system.dark).toBe(true);
    const stop = system.start();
    media.set(false);
    expect(system.dark).toBe(false);
    stop();
    expect(media.listeners.size).toBe(0);
  });

  it("follows Tauri's theme event", async () => {
    fakeMatchMedia(false);
    const system = new SystemMode();
    const stop = system.start();
    emit('systemTheme', true);
    expect(system.dark).toBe(true);
    emit('systemTheme', false);
    expect(system.dark).toBe(false);
    stop();
  });
});
