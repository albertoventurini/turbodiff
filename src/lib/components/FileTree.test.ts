import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { changeSet } from '../test/fakeApi';
import { buildTree, orderedFiles } from '../fileTree';
import { makeStores } from '../test/stores';
import type { FileEntry } from '../types';
import FileTree from './FileTree.svelte';
import RepoView from './RepoView.svelte';
import Sidebar from './Sidebar.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => vi.resetAllMocks());

function entry(path: string, overrides: Partial<FileEntry> = {}): FileEntry {
  return {
    path,
    oldPath: null,
    status: 'M',
    similarity: null,
    additions: 1,
    deletions: 1,
    binary: false,
    modeChange: null,
    submodule: null,
    ...overrides,
  };
}

const files = [
  entry('src/app/core/main.rs', { additions: 12, deletions: 4 }),
  entry('src/app/core/util.rs', { status: 'A', additions: 40, deletions: 0 }),
  entry('docs/old.md', { status: 'D', additions: 0, deletions: 30 }),
  entry('assets/logo.png', { status: 'A', binary: true, additions: 0, deletions: 0 }),
  entry('README.md', { status: 'R', oldPath: 'README', similarity: 90 }),
];

function renderTree(list = files) {
  const stores = makeStores();
  render(FileTree, { props: { files: list }, context: stores.context });
  return stores;
}

/** The visible rows' text, in order. */
function rows(): string[] {
  return within(screen.getByRole('list', { name: 'Changed files' }))
    .getAllByRole('button')
    .map((row) => row.textContent!.replace(/\s+/g, ' ').trim());
}

describe('buildTree', () => {
  it('compacts chains of single-child folders and lists folders first', () => {
    const tree = buildTree(files);
    expect(tree.map((node) => node.name)).toEqual(['assets', 'docs', 'src/app/core', 'README.md']);
    const src = tree[2];
    expect(src.kind === 'folder' && src.path).toBe('src/app/core');
    expect(src.kind === 'folder' && src.children.map((node) => node.name)).toEqual([
      'main.rs',
      'util.rs',
    ]);
  });

  it('keeps a folder that holds a file and a folder', () => {
    const tree = buildTree([entry('a/b/c.txt'), entry('a/d.txt')]);
    expect(tree).toHaveLength(1);
    expect(tree[0].name).toBe('a');
  });

  it('orders the files for the diff list like the sidebar', () => {
    const paths = (mode: 'tree' | 'flat') => orderedFiles(files, mode).map((file) => file.path);
    expect(paths('tree')).toEqual([
      'assets/logo.png',
      'docs/old.md',
      'src/app/core/main.rs',
      'src/app/core/util.rs',
      'README.md',
    ]);
    expect(paths('flat')).toEqual([
      'assets/logo.png',
      'docs/old.md',
      'README.md',
      'src/app/core/main.rs',
      'src/app/core/util.rs',
    ]);
  });
});

describe('FileTree', () => {
  it('shows folders, status badges and line counts', () => {
    renderTree();
    expect(rows()).toEqual([
      'assets',
      'A logo.png bin',
      'docs',
      'D old.md −30',
      'src/app/core',
      'M main.rs +12 −4',
      'A util.rs +40',
      'R README.md +1 −1',
    ]);
    expect(screen.getByRole('button', { name: /README\.md/ })).toHaveAttribute(
      'title',
      'Renamed: README → README.md',
    );
  });

  it('collapses and expands a folder', async () => {
    renderTree();
    const folder = screen.getByRole('button', { name: 'src/app/core' });
    expect(folder).toHaveAttribute('aria-expanded', 'true');
    await fireEvent.click(folder);
    expect(folder).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('button', { name: /main\.rs/ })).toBeNull();
    await fireEvent.click(folder);
    expect(screen.getByRole('button', { name: /main\.rs/ })).toBeInTheDocument();
  });

  it('switches to a flat list of full paths', async () => {
    const stores = renderTree();
    stores.view.fileTreeMode = 'flat';
    await vi.waitFor(() =>
      expect(rows()).toEqual([
        'A assets/logo.png bin',
        'D docs/old.md −30',
        'R README.md +1 −1',
        'M src/app/core/main.rs +12 −4',
        'A src/app/core/util.rs +40',
      ]),
    );
  });

  it('selects a file and deselects it with a second click', async () => {
    const stores = renderTree();
    const main = screen.getByRole('button', { name: /main\.rs/ });
    await fireEvent.click(main);
    expect(stores.view.selected).toBe('src/app/core/main.rs');
    expect(main).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(main);
    expect(stores.view.selected).toBeNull();
    expect(main).toHaveAttribute('aria-pressed', 'false');
  });

  it('marks the current file in the all-files view only', async () => {
    const stores = renderTree();
    stores.view.current = 'docs/old.md';
    const old = screen.getByRole('button', { name: /old\.md/ });
    await vi.waitFor(() => expect(old).toHaveAttribute('aria-current', 'true'));
    stores.view.selected = 'src/app/core/main.rs';
    await vi.waitFor(() => expect(old).not.toHaveAttribute('aria-current'));
  });
});

describe('Sidebar', () => {
  it('shows the summary line and the tree / flat toggle', async () => {
    const stores = makeStores();
    stores.session.changes = changeSet({
      files,
      totals: { files: 5, additions: 54, deletions: 35 },
    });
    render(Sidebar, { context: stores.context });
    expect(screen.getByTestId('summary')).toHaveTextContent('5 files changed +54 −35');
    await fireEvent.click(screen.getByRole('button', { name: 'Flat' }));
    expect(stores.view.fileTreeMode).toBe('flat');
    expect(screen.getByRole('button', { name: 'Flat' })).toHaveAttribute('aria-pressed', 'true');
  });
});

describe('Esc', () => {
  function setup() {
    const stores = makeStores();
    stores.session.repo = { root: '/work/app', name: 'app', head: { branch: 'main', commit: 'a' } };
    stores.session.changes = changeSet({ files });
    render(RepoView, { context: stores.context });
    return stores;
  }

  it('goes back to all files', async () => {
    const stores = setup();
    await fireEvent.click(screen.getByRole('button', { name: /main\.rs/ }));
    expect(stores.view.selected).toBe('src/app/core/main.rs');
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(stores.view.selected).toBeNull();
  });

  it('deselects a file that is no longer changed', async () => {
    const stores = setup();
    stores.view.selected = 'docs/old.md';
    stores.session.changes = changeSet({ files: files.slice(0, 2) });
    await vi.waitFor(() => expect(stores.view.selected).toBeNull());
  });
});
