import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { changeSet } from '../test/fakeApi';
import { defaultComparison } from '../stores/session.svelte';
import { makeStores } from '../test/stores';
import type { FileEntry } from '../types';
import MainPane from './MainPane.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => vi.resetAllMocks());

const file: FileEntry = {
  path: 'src/main.rs',
  oldPath: null,
  status: 'M',
  similarity: null,
  additions: 1,
  deletions: 1,
  binary: false,
  modeChange: null,
  submodule: null,
};

const branches = {
  base: 'refs/heads/main',
  compare: { kind: 'commit', spec: 'refs/heads/feature' },
} as const;

function setup(fill: (stores: ReturnType<typeof makeStores>) => void) {
  const stores = makeStores();
  fill(stores);
  render(MainPane, { context: stores.context });
  return stores;
}

describe('MainPane', () => {
  it('says the working tree matches HEAD when nothing changed', () => {
    setup(({ session }) => (session.changes = changeSet()));
    expect(screen.getByRole('heading', { name: 'No changes' })).toBeInTheDocument();
    expect(screen.getByText('Working tree matches HEAD.')).toBeInTheDocument();
  });

  it('names the refs when nothing changed between them', () => {
    setup(({ session }) => {
      session.comparison = { ...defaultComparison(), ...branches };
      session.changes = changeSet();
    });
    expect(screen.getByText(/Nothing has changed on/)).toHaveTextContent(
      'Nothing has changed on feature since it split off from main.',
    );
  });

  it('shows an invalid ref and goes back to the default comparison', async () => {
    const stores = setup(({ session }) => {
      session.comparison = { ...defaultComparison(), ...branches };
      session.error = { kind: 'refNotFound', name: 'refs/heads/gone' };
    });
    expect(screen.getByRole('heading', { name: 'Invalid ref' })).toBeInTheDocument();
    expect(screen.getByText(/isn't a branch, tag or commit/)).toHaveTextContent('refs/heads/gone');

    api.setComparison.mockResolvedValue(changeSet({ generation: 2 }));
    await fireEvent.click(screen.getByRole('button', { name: 'Compare working tree with HEAD' }));
    expect(api.setComparison).toHaveBeenCalledWith(defaultComparison());
    await vi.waitFor(() => expect(stores.session.error).toBeNull());
    expect(await screen.findByText('Working tree matches HEAD.')).toBeInTheDocument();
  });

  it('tells when there is no merge base', () => {
    setup(({ session }) => {
      session.comparison = { ...defaultComparison(), ...branches };
      session.changes = changeSet({
        files: [file],
        totals: { files: 1, additions: 1, deletions: 1 },
        noMergeBase: true,
      });
    });
    expect(screen.getByRole('status')).toHaveTextContent(
      'No merge base. main and feature have no common history, so this is a direct (two-dot) comparison.',
    );
    expect(screen.getByTestId('diff-placeholder')).toHaveTextContent('1 changed files');
  });

  it('offers to retry after another error', async () => {
    setup(({ session }) => (session.error = { kind: 'io', message: 'permission denied' }));
    expect(screen.getByRole('heading', { name: "Couldn't list the changes" })).toBeInTheDocument();
    expect(screen.getByText('permission denied')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(api.setComparison).toHaveBeenCalledWith(defaultComparison());
  });

  it('shows loading only while there is nothing else to show', () => {
    setup(({ session }) => (session.loading = true));
    expect(screen.getByRole('status')).toHaveTextContent('Loading changes…');
  });
});
