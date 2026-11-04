import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { changeSet, versioned } from '../test/fakeApi';
import { selectionTab } from '../refs';
import { makeStores } from '../test/stores';
import type { CommitItem, CompareSide, Comparison, RefItem } from '../types';
import RefPicker from './RefPicker.svelte';
import Toolbar from './Toolbar.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

const NOW = Date.now() / 1000;

function ref(name: string, kind: RefItem['kind'], overrides: Partial<RefItem> = {}): RefItem {
  const prefix = { localBranch: 'refs/heads/', remoteBranch: 'refs/remotes/', tag: 'refs/tags/' };
  return {
    name,
    fullName: prefix[kind] + name,
    kind,
    commit: 'c'.repeat(40),
    current: false,
    time: NOW - 3600,
    ...overrides,
  };
}

function commit(n: number, subject = `Commit ${n}`): CommitItem {
  const sha = n.toString(16).padStart(40, '0');
  return { sha, shortSha: sha.slice(0, 7), subject, authorName: 'Ada', time: NOW - n * 60 };
}

const branches = [
  ref('main', 'localBranch', { current: true }),
  ref('feature/login', 'localBranch'),
  ref('origin/main', 'remoteBranch'),
];
const tags = [ref('v1.0', 'tag'), ref('v0.2', 'tag')];
const head = { branch: 'main', commit: 'c'.repeat(40) };

beforeEach(() => {
  vi.resetAllMocks();
  api.listBranches.mockResolvedValue(versioned(branches));
  api.listTags.mockResolvedValue(versioned(tags));
  api.listCommits.mockImplementation((offset, limit) =>
    Promise.resolve(
      versioned(
        Array.from({ length: Math.max(0, Math.min(limit, 120 - offset)) }, (_, i) =>
          commit(offset + i + 1),
        ),
      ),
    ),
  );
});

function setup(side: 'base' | 'compare', value: CompareSide) {
  const stores = makeStores();
  stores.session.repo = { root: '/work/app', name: 'app', head };
  const onselect = vi.fn();
  render(RefPicker, { props: { side, value, onselect }, context: stores.context });
  return { stores, onselect };
}

async function open(side: 'base' | 'compare') {
  await fireEvent.click(screen.getByRole('button', { name: new RegExp(`^${side}:`) }));
  const dialog = await screen.findByRole('dialog', { name: `Choose the ${side}` });
  // Wait for the branch and tag lists.
  await vi.waitFor(() => expect(within(dialog).queryByText('Loading…')).toBeNull());
  return within(dialog);
}

/** The names of the listed items, in order. */
function items(dialog: ReturnType<typeof within>): string[] {
  return within(dialog.getByRole('tabpanel'))
    .queryAllByRole('button')
    .map((item) => item.title);
}

describe('RefPicker', () => {
  it('lists branches with the current one marked, and switches tabs', async () => {
    setup('base', { kind: 'commit', spec: 'HEAD' });
    const dialog = await open('base');
    expect(dialog.getByRole('tab', { name: 'Branches' })).toHaveAttribute('aria-selected', 'true');
    expect(items(dialog)).toEqual([
      'refs/heads/main',
      'refs/heads/feature/login',
      'refs/remotes/origin/main',
    ]);
    // HEAD is on main, so main is the selection.
    expect(dialog.getByRole('button', { name: /^main current/ })).toHaveAttribute(
      'aria-current',
      'true',
    );

    await fireEvent.click(dialog.getByRole('tab', { name: 'Tags' }));
    expect(items(dialog)).toEqual(['refs/tags/v1.0', 'refs/tags/v0.2']);
  });

  it('pins the working tree in the compare picker only', async () => {
    const { onselect } = setup('compare', { kind: 'commit', spec: 'refs/heads/feature/login' });
    let dialog = await open('compare');
    await fireEvent.click(dialog.getByRole('button', { name: 'Working tree' }));
    expect(onselect).toHaveBeenCalledWith({ kind: 'workTree' });
    expect(screen.queryByRole('dialog')).toBeNull();

    document.body.innerHTML = '';
    setup('base', { kind: 'commit', spec: 'HEAD' });
    dialog = await open('base');
    expect(dialog.queryByRole('button', { name: 'Working tree' })).toBeNull();
  });

  it('filters the active tab, ignoring case', async () => {
    setup('base', { kind: 'commit', spec: 'HEAD' });
    const dialog = await open('base');
    const search = dialog.getByRole('textbox', { name: 'Find a branch, tag or commit' });
    expect(search).toHaveFocus();
    await fireEvent.input(search, { target: { value: 'MAIN' } });
    expect(items(dialog)).toEqual(['refs/heads/main', 'refs/remotes/origin/main']);
    await fireEvent.input(search, { target: { value: 'nothing' } });
    expect(dialog.getByText('Nothing matches “nothing”.')).toBeInTheDocument();
  });

  it('picks a branch by its full name', async () => {
    const { onselect } = setup('base', { kind: 'commit', spec: 'HEAD' });
    const dialog = await open('base');
    await fireEvent.click(dialog.getByRole('button', { name: /^feature\/login/ }));
    expect(onselect).toHaveBeenCalledWith({ kind: 'commit', spec: 'refs/heads/feature/login' });
  });

  it('opens on the tab of the current selection', async () => {
    setup('base', { kind: 'commit', spec: 'refs/tags/v1.0' });
    const dialog = await open('base');
    expect(dialog.getByRole('tab', { name: 'Tags' })).toHaveAttribute('aria-selected', 'true');
    expect(dialog.getByRole('button', { name: /^v1\.0/ })).toHaveAttribute('aria-current', 'true');
  });

  it('loads more commits as the list scrolls', async () => {
    setup('base', { kind: 'commit', spec: commit(3).sha });
    const dialog = await open('base');
    expect(dialog.getByRole('tab', { name: 'Commits' })).toHaveAttribute('aria-selected', 'true');
    await vi.waitFor(() => expect(items(dialog)).toHaveLength(50));
    expect(api.listCommits).toHaveBeenCalledWith(0, 50);
    expect(dialog.getByRole('button', { name: /Commit 3\b/ })).toHaveAttribute(
      'aria-current',
      'true',
    );

    const list = dialog.getByRole('tabpanel');
    Object.defineProperties(list, {
      scrollHeight: { value: 2000, configurable: true },
      clientHeight: { value: 300, configurable: true },
    });
    list.scrollTop = 1600;
    await fireEvent.scroll(list);
    await vi.waitFor(() => expect(items(dialog)).toHaveLength(100));
    expect(api.listCommits).toHaveBeenLastCalledWith(50, 50);

    list.scrollTop = 1600;
    await fireEvent.scroll(list);
    await vi.waitFor(() => expect(items(dialog)).toHaveLength(120));
    // The last page was short, so there's nothing more to load.
    await fireEvent.scroll(list);
    expect(api.listCommits).toHaveBeenCalledTimes(3);
  });

  it('searches commit messages on the Commits tab', async () => {
    api.searchCommits.mockResolvedValue(versioned([commit(10, 'Fix bug number 10')]));
    setup('base', { kind: 'commit', spec: 'HEAD' });
    const dialog = await open('base');
    await fireEvent.click(dialog.getByRole('tab', { name: 'Commits' }));
    await fireEvent.input(dialog.getByRole('textbox'), { target: { value: 'fix bug' } });
    await vi.waitFor(() => expect(items(dialog)).toEqual([commit(10).sha]));
    expect(api.searchCommits).toHaveBeenCalledWith('fix bug', 50);
    expect(api.resolveCommitPrefix).not.toHaveBeenCalled();
  });

  it('finds a pasted SHA that is not in the loaded list', async () => {
    const pasted = { ...commit(500, 'Old work'), sha: 'abcdef1' + '0'.repeat(33) };
    api.resolveCommitPrefix.mockResolvedValue(versioned([pasted]));
    const { onselect } = setup('base', { kind: 'commit', spec: 'HEAD' });
    const dialog = await open('base');
    await fireEvent.input(dialog.getByRole('textbox'), { target: { value: 'ABCDEF1' } });
    // Offered on the Branches tab too.
    await vi.waitFor(() => expect(dialog.getByText('Commits with this SHA')).toBeInTheDocument());
    expect(api.resolveCommitPrefix).toHaveBeenCalledWith('ABCDEF1');

    await fireEvent.click(dialog.getByRole('tab', { name: 'Commits' }));
    await vi.waitFor(() => expect(items(dialog)[0]).toBe(pasted.sha));
    await fireEvent.click(dialog.getByRole('button', { name: /Old work/ }));
    expect(onselect).toHaveBeenCalledWith({ kind: 'commit', spec: pasted.sha });
  });

  it('drops lists from an older generation', async () => {
    const { stores } = setup('base', { kind: 'commit', spec: 'HEAD' });
    stores.session.generation = 3;
    api.listBranches.mockResolvedValue(versioned(branches, 2));
    const dialog = await open('base');
    expect(items(dialog)).toEqual([]);
  });

  it('closes on Esc without deselecting a file', async () => {
    const stores = makeStores();
    stores.session.repo = { root: '/work/app', name: 'app', head };
    stores.view.selected = 'a.txt';
    render(RefPicker, {
      props: { side: 'base', value: { kind: 'commit', spec: 'HEAD' }, onselect: () => {} },
      context: stores.context,
    });
    const keydowns = vi.fn((e: KeyboardEvent) => e.defaultPrevented);
    window.addEventListener('keydown', keydowns);
    const dialog = await open('base');
    await fireEvent.keyDown(dialog.getByRole('textbox'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(keydowns).toHaveReturnedWith(true);
    expect(screen.getByRole('button', { name: /^base:/ })).toHaveFocus();
    window.removeEventListener('keydown', keydowns);
  });
});

describe('selectionTab', () => {
  it('finds the tab from the ref name or the lists', () => {
    expect(selectionTab('refs/heads/main', head, null, null)).toBe('branches');
    expect(selectionTab('refs/remotes/origin/main', head, null, null)).toBe('branches');
    expect(selectionTab('refs/tags/v1.0', head, null, null)).toBe('tags');
    expect(selectionTab('HEAD', head, null, null)).toBe('branches');
    expect(selectionTab('HEAD', { branch: null, commit: 'a' }, null, null)).toBe('commits');
    expect(selectionTab('a'.repeat(40), head, branches, tags)).toBe('commits');
    // Short names from the command line.
    expect(selectionTab('v1.0', head, branches, tags)).toBe('tags');
    expect(selectionTab('main', head, branches, tags)).toBe('branches');
    expect(selectionTab('main~3', head, branches, tags)).toBe('commits');
    expect(selectionTab('main~3', head, null, null)).toBe('branches');
  });
});

describe('Toolbar', () => {
  const feature: Comparison = {
    base: 'refs/heads/main',
    compare: { kind: 'commit', spec: 'refs/heads/feature/login' },
    mode: 'threeDot',
    ignoreWhitespace: false,
  };

  function renderToolbar(comparison: Comparison) {
    const stores = makeStores();
    stores.session.repo = { root: '/work/app', name: 'app', head };
    stores.session.comparison = comparison;
    api.setComparison.mockResolvedValue(changeSet());
    render(Toolbar, { context: stores.context });
    return stores;
  }

  it('changes the base through the session', async () => {
    renderToolbar(feature);
    const dialog = await open('base');
    await fireEvent.click(dialog.getByRole('tab', { name: 'Tags' }));
    await fireEvent.click(dialog.getByRole('button', { name: /^v0\.2/ }));
    expect(api.setComparison).toHaveBeenCalledWith({ ...feature, base: 'refs/tags/v0.2' });
  });

  it('swaps base and compare, except for the working tree', async () => {
    const stores = renderToolbar(feature);
    const swap = screen.getByRole('button', { name: 'Swap base and compare' });
    await fireEvent.click(swap);
    expect(api.setComparison).toHaveBeenCalledWith({
      ...feature,
      base: 'refs/heads/feature/login',
      compare: { kind: 'commit', spec: 'refs/heads/main' },
    });

    stores.session.comparison = { ...feature, compare: { kind: 'workTree' } };
    await vi.waitFor(() => expect(swap).toBeDisabled());
  });

  it('toggles three-dot and two-dot, with an explanation', async () => {
    renderToolbar(feature);
    const group = screen.getByRole('group', { name: 'Comparison mode' });
    expect(group.title).toContain('Three-dot (•••): only the changes made on compare');
    const twoDot = within(group).getByRole('button', { name: 'Two-dot: direct comparison' });
    await fireEvent.click(twoDot);
    expect(api.setComparison).toHaveBeenCalledWith({ ...feature, mode: 'twoDot' });
    expect(twoDot).toHaveAttribute('aria-pressed', 'true');
  });
});
