import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { DEFAULT_SETTINGS } from '../stores/settings.svelte';
import { makeStores } from '../test/stores';
import type { Settings } from '../types';
import SettingsWindow from './SettingsWindow.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => {
  vi.resetAllMocks();
  api.listMonospaceFonts.mockResolvedValue(['DejaVu Sans Mono', 'Fira Code']);
});

function setup(settings: Partial<Settings> = {}) {
  const stores = makeStores();
  stores.settings.value = { ...DEFAULT_SETTINGS, ...settings };
  render(SettingsWindow, { context: stores.context });
  return stores;
}

function pressed(group: string): string | undefined {
  const buttons = within(screen.getByRole('group', { name: group })).getAllByRole('button');
  return buttons.find((b) => b.getAttribute('aria-pressed') === 'true')?.textContent?.trim();
}

describe('SettingsWindow', () => {
  it('shows the current settings', async () => {
    setup({ appearance: 'dark', diffStyle: 'split', tabWidth: 8, lineWrapping: true });
    expect(pressed('Appearance')).toBe('Dark');
    expect(pressed('Default diff style')).toBe('Side-by-side');
    expect(pressed('Diff algorithm')).toBe('Histogram');
    expect(pressed('Line wrapping')).toBe('On');
    expect(pressed('Tab width')).toBe('8');
    expect(screen.getByLabelText('Font size')).toHaveValue(13);
    expect(screen.getByLabelText('Context lines')).toHaveValue(3);
    expect(screen.getByLabelText('Theme')).toBeDisabled();
    expect(screen.getByLabelText('Theme')).toHaveDisplayValue('GitHub');
    await screen.findByRole('option', { name: 'Fira Code' });
    expect(screen.getByLabelText('Code font')).toHaveDisplayValue('Default monospace');
  });

  it('saves each change right away', async () => {
    const stores = setup();
    await fireEvent.click(screen.getByRole('button', { name: 'Light' }));
    expect(api.setSettings).toHaveBeenLastCalledWith({ appearance: 'light' });
    await fireEvent.click(screen.getByRole('button', { name: 'Side-by-side' }));
    expect(api.setSettings).toHaveBeenLastCalledWith({ diffStyle: 'split' });
    await fireEvent.click(screen.getByRole('button', { name: 'Myers' }));
    expect(api.setSettings).toHaveBeenLastCalledWith({ diffAlgorithm: 'myers' });
    await fireEvent.click(screen.getByRole('button', { name: 'On' }));
    expect(api.setSettings).toHaveBeenLastCalledWith({ lineWrapping: true });
    await fireEvent.click(screen.getByRole('button', { name: '2' }));
    expect(api.setSettings).toHaveBeenLastCalledWith({ tabWidth: 2 });

    await screen.findByRole('option', { name: 'Fira Code' });
    await fireEvent.change(screen.getByLabelText('Code font'), { target: { value: 'Fira Code' } });
    expect(api.setSettings).toHaveBeenLastCalledWith({ codeFont: 'Fira Code' });

    await fireEvent.change(screen.getByLabelText('Font size'), { target: { value: '16' } });
    expect(api.setSettings).toHaveBeenLastCalledWith({ fontSize: 16 });
    await vi.waitFor(() => expect(stores.settings.value.fontSize).toBe(16));
  });

  it('clamps numbers and puts back text that is not one', async () => {
    setup();
    const size = screen.getByLabelText<HTMLInputElement>('Font size');
    await fireEvent.change(size, { target: { value: '99' } });
    expect(api.setSettings).toHaveBeenLastCalledWith({ fontSize: 24 });
    expect(size.value).toBe('24');

    const context = screen.getByLabelText<HTMLInputElement>('Context lines');
    await fireEvent.change(context, { target: { value: '-4' } });
    expect(api.setSettings).toHaveBeenLastCalledWith({ contextLines: 0 });

    api.setSettings.mockClear();
    await fireEvent.change(context, { target: { value: '' } });
    expect(context.value).toBe('0');
    expect(api.setSettings).not.toHaveBeenCalled();
  });

  it('keeps a saved font that is not installed', async () => {
    setup({ codeFont: 'Iosevka' });
    expect(
      await screen.findByRole('option', { name: 'Iosevka (not installed)' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Code font')).toHaveDisplayValue('Iosevka (not installed)');
  });

  it('offers only the default font when the list fails to load', async () => {
    api.listMonospaceFonts.mockRejectedValue(new Error('no fonts'));
    setup();
    await vi.waitFor(() => expect(api.listMonospaceFonts).toHaveBeenCalled());
    const options = within(screen.getByLabelText('Code font')).getAllByRole('option');
    expect(options.map((o) => o.textContent)).toEqual(['Default monospace']);
  });

  it('resets to the defaults', async () => {
    const stores = setup({ fontSize: 20 });
    await fireEvent.click(screen.getByRole('button', { name: 'Reset to defaults' }));
    expect(api.resetSettings).toHaveBeenCalled();
    await vi.waitFor(() => expect(stores.settings.value.fontSize).toBe(13));
    expect(screen.getByLabelText('Font size')).toHaveValue(13);
  });
});
