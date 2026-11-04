import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { DEFAULT_SETTINGS } from './lib/stores/settings.svelte';
import { defaultComparison } from './lib/stores/session.svelte';
import * as api from './lib/test/fakeApi';
import { changeSet, emit } from './lib/test/fakeApi';

vi.mock('./lib/api', () => import('./lib/test/fakeApi'));

const root = document.documentElement;

beforeEach(() => {
  vi.resetAllMocks();
  api.getSettings.mockResolvedValue(DEFAULT_SETTINGS);
  api.getWindowState.mockResolvedValue({ repo: null, error: null });
  api.listRecent.mockResolvedValue([]);
});

afterEach(() => {
  delete root.dataset.theme;
  delete root.dataset.mode;
  window.history.replaceState(null, '', '/');
});

describe('App', () => {
  it('shows the welcome screen without a repository', async () => {
    render(App);
    expect(screen.getByRole('heading', { name: 'TurboDiff' })).toBeInTheDocument();
    await vi.waitFor(() => expect(api.getWindowState).toHaveBeenCalled());
  });

  it('shows the repository the window has', async () => {
    api.getWindowState.mockResolvedValue({
      repo: {
        repo: { root: '/work/app', name: 'app', head: { branch: 'main', commit: 'a' } },
        comparison: defaultComparison(),
      },
      error: null,
    });
    api.setComparison.mockResolvedValue(changeSet());
    render(App);
    expect(await screen.findByRole('heading', { name: 'app' })).toBeInTheDocument();
    expect(await screen.findByText('Working tree matches HEAD.')).toBeInTheDocument();
  });

  it('applies the appearance setting and follows changes live', async () => {
    window.__TURBODIFF_SETTINGS__ = { ...DEFAULT_SETTINGS, appearance: 'dark' };
    api.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance: 'dark' });
    render(App);
    delete window.__TURBODIFF_SETTINGS__;
    expect(root.dataset.theme).toBe('github');
    expect(root.dataset.mode).toBe('dark');

    await vi.waitFor(() => expect(api.getSettings).toHaveBeenCalled());
    emit('settingsChanged', { ...DEFAULT_SETTINGS, appearance: 'light' });
    await vi.waitFor(() => expect(root.dataset.mode).toBe('light'));

    // System follows the OS, and keeps following it.
    emit('settingsChanged', { ...DEFAULT_SETTINGS, appearance: 'system' });
    emit('systemTheme', true);
    await vi.waitFor(() => expect(root.dataset.mode).toBe('dark'));
    emit('systemTheme', false);
    await vi.waitFor(() => expect(root.dataset.mode).toBe('light'));

    // An explicit mode ignores the OS.
    emit('settingsChanged', { ...DEFAULT_SETTINGS, appearance: 'dark' });
    emit('systemTheme', false);
    await vi.waitFor(() => expect(root.dataset.mode).toBe('dark'));
  });

  it('shows the Settings placeholder in the Settings window', () => {
    window.history.replaceState(null, '', '/?window=settings');
    render(App);
    expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    expect(api.getWindowState).not.toHaveBeenCalled();
  });
});
