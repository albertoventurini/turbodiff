import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { ApiError, changeSet, versioned } from '../test/fakeApi';
import { makeStores } from '../test/stores';
import type { FileDiff, FileEntry } from '../types';
import DiffList from './DiffList.svelte';

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

/** Line 50 of 100 changed, with one line of context. */
const middle: FileDiff = {
  kind: 'text',
  hunks: [
    {
      oldStart: 49,
      oldLines: 3,
      newStart: 49,
      newLines: 3,
      lines: [
        { kind: 'ctx', oldNo: 49, newNo: 49, text: 'line 49', words: null },
        { kind: 'del', oldNo: 50, newNo: null, text: 'old fifty', words: [[0, 3]] },
        { kind: 'add', oldNo: null, newNo: 50, text: 'new fifty', words: [[0, 3]] },
        { kind: 'ctx', oldNo: 51, newNo: 51, text: 'line 51', words: null },
      ],
    },
  ],
  oldLineCount: 100,
  newLineCount: 100,
  noNewlineAtEof: { old: false, new: false },
};

function setup(files: FileEntry[], diffs: Record<string, FileDiff> = {}) {
  api.getFileDiff.mockImplementation((path: string) =>
    Promise.resolve(versioned(diffs[path] ?? middle, 1)),
  );
  const stores = makeStores();
  stores.session.changes = changeSet({ generation: 1, files });
  stores.session.generation = 1;
  render(DiffList, { context: stores.context });
  return stores;
}

function card(path: string): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>('.row')].filter((row) =>
    row.dataset.key!.startsWith(`${path}\0`),
  );
}

describe('DiffList', () => {
  it('lists the files in the sidebar order and loads their diffs', async () => {
    setup([entry('z.txt'), entry('src/a.txt'), entry('b.txt')]);
    const headers = screen
      .getAllByRole('button', { name: 'Copy path' })
      .map((button) => button.parentElement!.textContent!.replace(/\s+/g, ' ').trim());
    // Folders first, as in the file tree.
    expect(headers.map((text) => text.split(' ')[1])).toEqual(['src/a.txt', 'b.txt', 'z.txt']);

    await vi.waitFor(() => expect(api.getFileDiff).toHaveBeenCalledTimes(3));
    expect(api.getFileDiff).toHaveBeenCalledWith('src/a.txt', false);
    expect((await screen.findAllByText('fifty')).length).toBeGreaterThan(0);
  });

  it('marks the file at the top as current', async () => {
    const { view } = setup([entry('b.txt'), entry('a.txt')]);
    await vi.waitFor(() => expect(view.current).toBe('a.txt'));
  });

  it('renders inline lines with markers, word ranges and line numbers outside the text', async () => {
    setup([entry('a.txt')]);
    const deleted = (await screen.findByText('old', { selector: '.word' })).closest('.line')!;
    expect(deleted.querySelector('.code')).toHaveAttribute('data-marker', '−');
    // Exactly the line, so copying adds nothing.
    expect(deleted.querySelector('.code')!.textContent!.trim()).toBe('old fifty');
    expect(deleted.querySelector('.content')!.textContent).toBe('old fifty');
    expect([...deleted.querySelectorAll('.num')].map((num) => num.getAttribute('data-no'))).toEqual(
      ['50', ''],
    );
  });

  it('renders the split view with filler cells', async () => {
    const stores = setup([entry('a.txt')], {
      'a.txt': {
        ...middle,
        hunks: [
          {
            oldStart: 1,
            oldLines: 1,
            newStart: 1,
            newLines: 2,
            lines: [
              { kind: 'ctx', oldNo: 1, newNo: 1, text: 'same', words: null },
              { kind: 'add', oldNo: null, newNo: 2, text: 'added', words: null },
            ],
          },
        ],
      },
    });
    stores.view.diffStyle = 'split';
    const added = (await screen.findByText('added')).closest('.split')!;
    expect(added.querySelectorAll('.filler')).toHaveLength(2);
    expect(added.querySelector('[data-side="right"]')).toHaveTextContent('added');
  });

  it('collapses and expands a file', async () => {
    const { view } = setup([entry('a.txt')]);
    await screen.findByText('line 49');
    await fireEvent.click(screen.getByRole('button', { name: 'Collapse file' }));
    expect(view.collapsed.has('a.txt')).toBe(true);
    expect(screen.queryByText('line 49')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Expand file' }));
    expect(await screen.findByText('line 49')).toBeInTheDocument();
  });

  it('expands hidden lines with get_lines', async () => {
    setup([entry('a.txt')]);
    await screen.findByText('line 49');
    api.getLines.mockResolvedValue(
      versioned(
        Array.from({ length: 20 }, (_, i) => `line ${29 + i}`),
        1,
      ),
    );
    await fireEvent.click(screen.getByRole('button', { name: 'Expand up' }));
    expect(api.getLines).toHaveBeenCalledWith('a.txt', 'new', 29, 48);
    expect(await screen.findByText('line 29')).toBeInTheDocument();
    // Lines 1–28 are still hidden.
    expect(screen.getByRole('button', { name: 'Expand down' })).toBeInTheDocument();
  });

  it('loads a large diff on request', async () => {
    setup([entry('big.txt', { additions: 5000 })], {
      'big.txt': { kind: 'tooLarge', additions: 5000, deletions: 0 },
    });
    const load = await screen.findByRole('button', { name: 'Load diff' });
    api.getFileDiff.mockResolvedValue(versioned(middle, 1));
    await fireEvent.click(load);
    expect(api.getFileDiff).toHaveBeenLastCalledWith('big.txt', true);
    expect(await screen.findByText('line 49')).toBeInTheDocument();
  });

  it('shows special files', async () => {
    setup(
      [
        entry('a.bin', { binary: true }),
        entry('run.sh', { modeChange: ['100644', '100755'], additions: 0, deletions: 0 }),
        entry('lib', { submodule: ['a'.repeat(40), 'b'.repeat(40)] }),
        entry('ws.txt', { additions: 0, deletions: 0 }),
        entry('new.txt', { status: 'R', oldPath: 'old.txt', similarity: 90 }),
      ],
      {
        'a.bin': { kind: 'binary', oldSize: 2048, newSize: 4096 },
        'run.sh': { ...middle, hunks: [] },
        'ws.txt': { kind: 'whitespaceOnly' },
      },
    );
    expect(await screen.findByText('Binary file not shown.')).toBeInTheDocument();
    expect(screen.getByText('2.0 KB → 4.0 KB')).toBeInTheDocument();
    expect(screen.getByText(/File mode changed/)).toHaveTextContent(
      'File mode changed 100644 → 100755',
    );
    expect(screen.getByText(/Submodule commit/)).toHaveTextContent(
      'Submodule commit aaaaaaa → bbbbbbb',
    );
    expect(screen.getByText('Whitespace-only changes.')).toBeInTheDocument();
    const renamed = card('new.txt')[0];
    expect(within(renamed).getByText('old.txt')).toBeInTheDocument();
    expect(renamed).toHaveTextContent('90% similar');
    // The submodule's commits come with the file list.
    expect(api.getFileDiff).not.toHaveBeenCalledWith('lib', false);
  });

  it('drops a diff computed for an older generation', async () => {
    const stores = setup([entry('a.txt')]);
    stores.session.generation = 2;
    api.getFileDiff.mockResolvedValue(versioned(middle, 1));
    await vi.waitFor(() => expect(api.getFileDiff).toHaveBeenCalled());
    await Promise.resolve();
    expect(screen.queryByText('line 49')).toBeNull();
  });

  it('says when a diff fails to load', async () => {
    api.getFileDiff.mockRejectedValue(new ApiError({ kind: 'io', message: 'boom' }));
    const stores = makeStores();
    stores.session.changes = changeSet({ files: [entry('a.txt')] });
    render(DiffList, { context: stores.context });
    expect(await screen.findByText("Couldn't load this diff.")).toBeInTheDocument();
  });

  it('shows only the selected file', async () => {
    const { view } = setup([entry('a.txt'), entry('b.txt')]);
    view.selected = 'b.txt';
    await vi.waitFor(() => expect(card('a.txt')).toHaveLength(0));
    expect(card('b.txt').length).toBeGreaterThan(0);
    view.selected = null;
    await vi.waitFor(() => expect(card('a.txt').length).toBeGreaterThan(0));
  });
});
