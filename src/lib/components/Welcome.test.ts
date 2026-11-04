import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { ApiError } from '../test/fakeApi';
import { makeStores } from '../test/stores';
import Welcome from './Welcome.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => vi.resetAllMocks());

function setup() {
  const stores = makeStores();
  stores.recent.items = [
    { path: '/work/app', name: 'app', available: true },
    { path: '/old/gone', name: 'gone', available: false },
  ];
  render(Welcome, { context: stores.context });
  return stores;
}

describe('Welcome', () => {
  it('opens the folder picker', async () => {
    setup();
    await fireEvent.click(screen.getByRole('button', { name: /Open Repository/ }));
    expect(api.showOpenDialog).toHaveBeenCalled();
    expect(screen.getByText(/drop a folder onto this window/)).toBeInTheDocument();
  });

  it('opens a recent repository', async () => {
    setup();
    const list = screen.getByRole('list');
    await fireEvent.click(within(list).getByRole('button', { name: /app/ }));
    expect(api.openRepo).toHaveBeenCalledWith('/work/app');
  });

  it('marks unavailable entries, which can only be removed', async () => {
    const stores = setup();
    const [available, unavailable] = screen.getAllByRole('listitem');
    expect(within(available).queryByText('Unavailable')).toBeNull();
    expect(within(available).queryByRole('button', { name: /Remove/ })).toBeNull();
    expect(within(unavailable).getByText('Unavailable')).toBeInTheDocument();
    expect(within(unavailable).queryByRole('button', { name: /^gone/ })).toBeNull();

    api.removeRecent.mockResolvedValue([stores.recent.items[0]]);
    await fireEvent.click(within(unavailable).getByRole('button', { name: /Remove gone/ }));
    expect(api.removeRecent).toHaveBeenCalledWith('/old/gone');
    await vi.waitFor(() => expect(screen.getAllByRole('listitem')).toHaveLength(1));
  });

  it('shows an error when a folder is not a repository', async () => {
    setup();
    api.openRepo.mockRejectedValue(new ApiError({ kind: 'notARepository', path: '/work/app' }));
    await fireEvent.click(within(screen.getByRole('list')).getByRole('button', { name: /app/ }));
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('This folder is not a git repository: /work/app');

    await fireEvent.click(within(alert).getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('hides the recent list when it is empty', () => {
    const stores = makeStores();
    render(Welcome, { context: stores.context });
    expect(screen.queryByText('Recent repositories')).toBeNull();
  });
});
