import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { emit } from '../test/fakeApi';
import { DEFAULT_SETTINGS, SettingsStore } from './settings.svelte';
import { RecentStore } from './recent.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => {
  vi.resetAllMocks();
  delete window.__TURBODIFF_SETTINGS__;
});

describe('SettingsStore', () => {
  it('starts from the boot script settings', () => {
    window.__TURBODIFF_SETTINGS__ = { ...DEFAULT_SETTINGS, sidebarWidth: 400 };
    expect(new SettingsStore().value.sidebarWidth).toBe(400);
    delete window.__TURBODIFF_SETTINGS__;
    expect(new SettingsStore().value).toEqual(DEFAULT_SETTINGS);
  });

  it('loads the settings and follows settings-changed', async () => {
    api.getSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, appearance: 'dark' });
    const settings = new SettingsStore();
    const stop = settings.start();
    await vi.waitFor(() => expect(settings.value.appearance).toBe('dark'));
    emit('settingsChanged', { ...DEFAULT_SETTINGS, appearance: 'light' });
    expect(settings.value.appearance).toBe('light');
    stop();
  });

  it('keeps an event that arrives before the loaded settings', async () => {
    let resolve!: (value: typeof DEFAULT_SETTINGS) => void;
    api.getSettings.mockReturnValue(new Promise((r) => (resolve = r)));
    const settings = new SettingsStore();
    const stop = settings.start();
    await vi.waitFor(() => expect(api.getSettings).toHaveBeenCalled());
    emit('settingsChanged', { ...DEFAULT_SETTINGS, fontSize: 20 });
    resolve(DEFAULT_SETTINGS);
    await Promise.resolve();
    expect(settings.value.fontSize).toBe(20);
    stop();
  });

  it('saves a patch', async () => {
    api.setSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, sidebarWidth: 320 });
    const settings = new SettingsStore();
    await settings.update({ sidebarWidth: 320 });
    expect(api.setSettings).toHaveBeenCalledWith({ sidebarWidth: 320 });
    expect(settings.value.sidebarWidth).toBe(320);
  });

  it('resets to the defaults', async () => {
    const settings = new SettingsStore();
    settings.value = { ...DEFAULT_SETTINGS, fontSize: 20, sidebarWidth: 320 };
    await settings.reset();
    expect(api.resetSettings).toHaveBeenCalled();
    expect(settings.value).toEqual({ ...DEFAULT_SETTINGS, sidebarWidth: 320 });
  });
});

describe('RecentStore', () => {
  it('loads the list, follows recent-changed and removes entries', async () => {
    const item = { path: '/work/app', name: 'app', available: false };
    api.listRecent.mockResolvedValue([item]);
    api.removeRecent.mockResolvedValue([]);
    const recent = new RecentStore();
    const stop = recent.start();
    await vi.waitFor(() => expect(recent.items).toEqual([item]));

    emit('recentChanged', [{ ...item, available: true }]);
    expect(recent.items[0].available).toBe(true);

    await recent.remove('/work/app');
    expect(api.removeRecent).toHaveBeenCalledWith('/work/app');
    expect(recent.items).toEqual([]);
    stop();
  });
});
