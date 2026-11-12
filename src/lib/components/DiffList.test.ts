import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { ApiError, changeSet, versioned } from '../test/fakeApi';
import { makeStores } from '../test/stores';
import type { FileDiff, FileEntry } from '../types';
import DiffList from './DiffList.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

// Highlighting colours the first three characters of each line: red on the
// old side, blue on the new, in the light theme; the dark theme's are darker.
const highlight = vi.hoisted(() => vi.fn());
vi.mock('../highlight/client', () => ({
  Highlighter: class {
    broken = false;
    highlight = highlight;
    stop() {}
  },
}));
const COLORS: Record<string, [string, string]> = {
  'github-light': ['red', 'blue'],
  'github-dark': ['darkred', 'darkblue'],
};
function fakeHighlight(text: string, _lang: string, theme: string) {
  const lines = text.split('\n');
  const color = COLORS[theme][text.startsWith('old') ? 0 : 1];
  return Promise.resolve({ styles: [`color: ${color}`], lines: lines.map(() => [[3, 0]]) });
}

beforeEach(() => {
  vi.resetAllMocks();
  highlight.mockImplementation(fakeHighlight);
});

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

  it('keeps loaded diffs and expanded lines across a refresh', async () => {
    const files = [entry('a.txt'), entry('b.txt')];
    const stores = setup(files);
    await vi.waitFor(() => expect(screen.getAllByText('line 49')).toHaveLength(2));
    api.getLines.mockResolvedValue(
      versioned(
        Array.from({ length: 20 }, (_, i) => `line ${29 + i}`),
        1,
      ),
    );
    await fireEvent.click(screen.getAllByRole('button', { name: 'Expand up' })[0]);
    expect(await screen.findByText('line 29')).toBeInTheDocument();
    api.getFileDiff.mockClear();

    // b.txt changed on disk: its new diff replaces the old one, which stays
    // on screen until then.
    const changed: FileDiff = {
      ...middle,
      hunks: [
        {
          ...middle.hunks[0],
          lines: middle.hunks[0].lines.map((line) =>
            line.kind === 'add' ? { ...line, text: 'newer fifty' } : line,
          ),
        },
      ],
    };
    api.getFileDiff.mockResolvedValue(versioned(changed, 2));
    stores.session.accept(2);
    stores.session.changes = changeSet({ generation: 2, files });
    stores.session.refresh = { generation: 2, filesChanged: ['b.txt'] };
    expect(card('b.txt').some((row) => row.textContent?.includes('new fifty'))).toBe(true);
    await vi.waitFor(() =>
      expect(card('b.txt').some((row) => row.textContent?.includes('newer fifty'))).toBe(true),
    );
    expect(api.getFileDiff).toHaveBeenCalledTimes(1);
    expect(api.getFileDiff).toHaveBeenCalledWith('b.txt', false);
    expect(screen.getByText('line 29')).toBeInTheDocument();
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

  describe('syntax highlighting', () => {
    function highlighted(diffStyle: 'inline' | 'split' = 'inline') {
      api.getLines.mockImplementation((_path, side) =>
        Promise.resolve(versioned(Array<string>(100).fill(side), 1)),
      );
      const stores = setup([entry('a.ts')], { 'a.ts': middle });
      stores.view.diffStyle = diffStyle;
      return stores;
    }

    /** The colour of the first run of the line holding `text`. */
    function color(text: string, side?: 'left' | 'right'): string {
      const cell = [...document.querySelectorAll<HTMLElement>('.cell')].find(
        (cell) => cell.textContent === text && (!side || cell.dataset.side === side),
      )!;
      return cell.querySelector<HTMLElement>('.content > span')?.style.color ?? '';
    }

    it('colours lines from the side they belong to', async () => {
      highlighted();
      await vi.waitFor(() => expect(color('new fifty')).toBe('blue'));
      expect(api.getLines).toHaveBeenCalledWith('a.ts', 'old', 1, 100);
      expect(api.getLines).toHaveBeenCalledWith('a.ts', 'new', 1, 100);
      expect(color('old fifty')).toBe('red');
      expect(color('line 49')).toBe('blue');
      // Word ranges are kept.
      expect(screen.getByText('old', { selector: '.word' }).style.color).toBe('red');
    });

    it('colours unchanged lines from the new side in split view', async () => {
      highlighted('split');
      await vi.waitFor(() => expect(color('line 49', 'right')).toBe('blue'));
      expect(color('line 49', 'left')).toBe('blue');
      expect(color('old fifty', 'left')).toBe('red');
    });

    it('follows the mode', async () => {
      const stores = highlighted();
      await vi.waitFor(() => expect(color('new fifty')).toBe('blue'));
      stores.settings.value = { ...stores.settings.value, appearance: 'dark' };
      await vi.waitFor(() => expect(color('new fifty')).toBe('darkblue'));
      expect(highlight).toHaveBeenCalledWith(expect.any(String), 'typescript', 'github-dark');
    });

    it('shows plain text until highlighting arrives, and if it fails', async () => {
      highlight.mockResolvedValue(null);
      highlighted();
      await vi.waitFor(() => expect(highlight).toHaveBeenCalledTimes(2));
      await new Promise((resolve) => setTimeout(resolve));
      expect(color('new fifty')).toBe('');
      expect(screen.getByText('new', { selector: '.word' })).not.toHaveAttribute('style');
    });

    it('leaves files in unknown languages plain', async () => {
      setup([entry('a.txt')]);
      await screen.findByText('line 49');
      expect(api.getLines).not.toHaveBeenCalled();
      expect(highlight).not.toHaveBeenCalled();
    });
  });

  describe('find in page', () => {
    const marked = () =>
      [...document.querySelectorAll('mark.found')].map((mark) => [
        mark.textContent,
        mark.classList.contains('current'),
      ]);

    async function find(query: string, files = [entry('a.txt'), entry('b.txt')]) {
      const stores = setup(files);
      await screen.findAllByText('line 49');
      stores.view.find();
      const input = await screen.findByRole('textbox', { name: 'Find in diff' });
      await fireEvent.input(input, { target: { value: query } });
      return { stores, input };
    }

    it('marks every match, and steps through them with Enter and Shift+Enter', async () => {
      const { input } = await find('FIFTY');
      await vi.waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('1 of 4'));
      expect(marked()).toEqual([
        ['fifty', true],
        ['fifty', false],
        ['fifty', false],
        ['fifty', false],
      ]);
      await fireEvent.keyDown(input, { key: 'Enter' });
      expect(screen.getByRole('status')).toHaveTextContent('2 of 4');
      expect(marked()[1]).toEqual(['fifty', true]);
      await fireEvent.keyDown(input, { key: 'Enter', shiftKey: true });
      await fireEvent.keyDown(input, { key: 'Enter', shiftKey: true });
      expect(screen.getByRole('status')).toHaveTextContent('4 of 4');
      await fireEvent.click(screen.getByRole('button', { name: 'Next match' }));
      expect(screen.getByRole('status')).toHaveTextContent('1 of 4');
    });

    it('says when nothing matches', async () => {
      await find('nowhere');
      expect(screen.getByRole('status')).toHaveTextContent('No results');
      expect(screen.getByRole('button', { name: 'Next match' })).toBeDisabled();
    });

    it('loads every diff to search it, and leaves out collapsed files', async () => {
      // Tall enough that only the first files load on their own.
      const files = Array.from({ length: 30 }, (_, i) =>
        entry(`f${String(i).padStart(2, '0')}.txt`, { additions: 200 }),
      );
      const stores = setup(files);
      stores.view.collapsed.add('f29.txt');
      await vi.waitFor(() => expect(api.getFileDiff).toHaveBeenCalled());
      expect(api.getFileDiff).not.toHaveBeenCalledWith('f28.txt', false);
      stores.view.find();
      await fireEvent.input(await screen.findByRole('textbox', { name: 'Find in diff' }), {
        target: { value: 'old' },
      });
      await vi.waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('1 of 29'));
      expect(api.getFileDiff).toHaveBeenCalledWith('f28.txt', false);
      expect(api.getFileDiff).not.toHaveBeenCalledWith('f29.txt', false);
    });

    it('closes with Esc, keeping the query for next time', async () => {
      const { stores, input } = await find('fifty');
      const escape = await fireEvent.keyDown(input, { key: 'Escape' });
      expect(escape).toBe(false);
      expect(stores.view.findOpen).toBe(false);
      expect(screen.queryByRole('search')).toBeNull();
      expect(marked()).toEqual([]);
      stores.view.find();
      expect(await screen.findByRole('textbox', { name: 'Find in diff' })).toHaveValue('fifty');
    });
  });

  describe('Copy reference', () => {
    const writeText = vi.fn();
    beforeEach(() => {
      writeText.mockResolvedValue(undefined);
      Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    });

    function select(from: Node, fromOffset: number, to: Node, toOffset: number) {
      document.getSelection()!.setBaseAndExtent(from, fromOffset, to, toOffset);
      document.dispatchEvent(new Event('selectionchange'));
    }

    /** The row whose (first) line is `text`. */
    async function rowWith(text: string): Promise<Element> {
      return vi.waitFor(() => {
        const row = [...document.querySelectorAll('.row')].find(
          (row) => row.querySelector('.content')?.textContent === text,
        );
        expect(row).toBeDefined();
        return row!;
      });
    }

    it('copies the selected parts of lines with their place and the comparison', async () => {
      setup([entry('src/a.txt')]);
      const deleted = await rowWith('old fifty');
      const added = screen.getByText('new', { selector: '.word' }).closest('.row')!;
      // From "fifty" on the deleted line to "new" on the added one.
      const rest = [...deleted.querySelector('.content')!.childNodes].find(
        (node) => node.textContent === ' fifty',
      )!;
      select(rest, 1, added.querySelector('.word')!.firstChild!, 3);
      await fireEvent.click(await screen.findByRole('button', { name: 'Copy reference' }));
      expect(writeText).toHaveBeenCalledWith(
        'src/a.txt:50 (old version: src/a.txt:50), HEAD → working tree:\n' +
          '```diff\n-fifty\n+new\n```\n',
      );
      expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();
    });

    it('keeps to the side the selection started on in split view', async () => {
      const stores = setup([entry('a.txt')]);
      stores.view.diffStyle = 'split';
      const first = (await rowWith('line 49')).querySelector('[data-side="right"] .content')!;
      const last = (await rowWith('line 51')).querySelector('[data-side="right"] .content')!;
      select(first, 0, last, last.childNodes.length);
      await fireEvent.click(await screen.findByRole('button', { name: 'Copy reference' }));
      expect(writeText).toHaveBeenCalledWith(
        'a.txt:49-51, HEAD → working tree:\n```diff\n line 49\n+new fifty\n line 51\n```\n',
      );
    });

    it('shows the button only for a selection of code', async () => {
      setup([entry('a.txt')]);
      const row = await rowWith('line 49');
      const content = row.querySelector('.content')!;
      select(content, 0, content, content.childNodes.length);
      expect(await screen.findByRole('button', { name: 'Copy reference' })).toBeInTheDocument();
      document.getSelection()!.removeAllRanges();
      document.dispatchEvent(new Event('selectionchange'));
      await vi.waitFor(() =>
        expect(screen.queryByRole('button', { name: 'Copy reference' })).toBeNull(),
      );
    });
  });
});
