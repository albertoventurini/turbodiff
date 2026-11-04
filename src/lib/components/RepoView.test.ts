import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { changeSet } from '../test/fakeApi';
import { DEFAULT_SETTINGS } from '../stores/settings.svelte';
import { makeStores } from '../test/stores';
import RepoView from './RepoView.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => vi.resetAllMocks());

function setup() {
  const stores = makeStores();
  stores.session.repo = { root: '/work/app', name: 'app', head: { branch: 'main', commit: 'a' } };
  stores.session.changes = changeSet();
  render(RepoView, { context: stores.context });
  return stores;
}

describe('RepoView', () => {
  it('shows the repository name and comparison in the toolbar', () => {
    setup();
    expect(screen.getByRole('heading', { name: 'app' })).toBeInTheDocument();
    expect(screen.getByTestId('comparison-bar')).toHaveTextContent(
      'base: HEAD compare: Working tree',
    );
  });

  it('collapses and expands the sidebar', async () => {
    setup();
    expect(screen.getByRole('complementary', { name: 'Changed files' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Hide sidebar' }));
    expect(screen.queryByRole('complementary')).toBeNull();
    expect(screen.queryByRole('separator')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Show sidebar' }));
    expect(screen.getByRole('complementary')).toBeInTheDocument();
  });

  it('starts at the saved sidebar width and saves a new one after a drag', async () => {
    const stores = setup();
    const sidebar = screen.getByRole('complementary');
    expect(sidebar.style.width).toBe(`${DEFAULT_SETTINGS.sidebarWidth}px`);

    const divider = screen.getByRole('separator', { name: 'Resize sidebar' });
    divider.setPointerCapture = () => {};
    api.setSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, sidebarWidth: 380 });
    await fireEvent.pointerDown(divider, { button: 0, clientX: 280, pointerId: 1 });
    await fireEvent.pointerMove(divider, { clientX: 380, pointerId: 1 });
    expect(sidebar.style.width).toBe('380px');
    expect(api.setSettings).not.toHaveBeenCalled();

    await fireEvent.pointerUp(divider, { clientX: 380, pointerId: 1 });
    expect(api.setSettings).toHaveBeenCalledWith({ sidebarWidth: 380 });
    await vi.waitFor(() => expect(stores.settings.value.sidebarWidth).toBe(380));
    expect(sidebar.style.width).toBe('380px');
  });

  it('keeps the sidebar width within its limits', async () => {
    setup();
    const divider = screen.getByRole('separator');
    divider.setPointerCapture = () => {};
    await fireEvent.pointerDown(divider, { button: 0, clientX: 280, pointerId: 1 });
    await fireEvent.pointerMove(divider, { clientX: 0, pointerId: 1 });
    expect(screen.getByRole('complementary').style.width).toBe('160px');
  });

  it('shows a failed open as a notice without leaving the repository', () => {
    const stores = setup();
    stores.session.notice = { kind: 'notARepository', path: '/tmp' };
    return vi.waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'This folder is not a git repository: /tmp',
      ),
    );
  });

  it('says which ref disappeared after falling back, until dismissed', async () => {
    const stores = setup();
    stores.session.fallback = 'refs/heads/feature';
    const notice = await screen.findByRole('status');
    expect(notice).toHaveTextContent(
      'feature no longer exists. Showing the working tree compared with HEAD.',
    );
    await fireEvent.click(within(notice).getByRole('button', { name: 'Dismiss' }));
    expect(stores.session.fallback).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('forgets collapsed files that left the list', async () => {
    const stores = setup();
    const file = (path: string) => ({
      path,
      oldPath: null,
      status: 'M' as const,
      similarity: null,
      additions: 1,
      deletions: 0,
      binary: false,
      modeChange: null,
      submodule: null,
    });
    stores.session.changes = changeSet({ files: [file('a.txt'), file('b.txt')] });
    stores.view.collapsed.add('a.txt');
    stores.view.collapsed.add('b.txt');
    stores.session.changes = changeSet({ generation: 2, files: [file('b.txt')] });
    await vi.waitFor(() => expect([...stores.view.collapsed]).toEqual(['b.txt']));
  });
});
