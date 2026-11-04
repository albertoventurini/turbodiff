// The diff view's rows (TECH.md §7.3): every file's header, hunk headers,
// lines, expand controls and messages, in one flat list that the virtual list
// renders. Inline rows hold one line; split rows hold an old and a new cell.
import type { Mark } from './find';
import type { DiffStyle, FileDiff, FileEntry, Hunk, Line, LineKind } from './types';

/** How many hidden lines one click on "expand up" or "expand down" reveals. */
export const EXPAND_STEP = 20;
/** Matches the backend's `LARGE_DIFF_LINES`, for estimating unloaded files. */
export const LARGE_DIFF_LINES = 1000;
/**
 * Matches the backend's `LARGE_FILE_BYTES`. Measured in UTF-16 code units
 * here, which is close enough for deciding what to highlight.
 */
export const LARGE_FILE_BYTES = 500_000;

/** A file's diff, once loaded, with the hidden lines revealed so far. */
export type LoadedDiff = {
  diff: FileDiff;
  /** Revealed unchanged lines, by new-side line number. */
  context: ReadonlyMap<number, string>;
};

/** Hidden unchanged lines: `start..=end` on the new side, `start + oldOffset` on the old. */
export type Gap = { start: number; end: number; oldOffset: number };

export type ExpandDirection = 'up' | 'down' | 'all';

/** One side of a split row, or an inline line. */
export type Cell = {
  kind: LineKind;
  no: number;
  text: string;
  words: [number, number][] | null;
  noNewline: boolean;
};

export type InlineLine = {
  kind: LineKind;
  oldNo: number | null;
  newNo: number | null;
  text: string;
  words: [number, number][] | null;
  noNewline: boolean;
};

export type Message =
  | { kind: 'tooLarge' }
  | { kind: 'binary'; oldSize: number | null; newSize: number | null }
  | { kind: 'whitespaceOnly' }
  | { kind: 'modeChange'; from: string; to: string }
  | { kind: 'submodule'; from: string; to: string }
  | { kind: 'noContent'; text: string }
  | { kind: 'failed'; text: string };

type Base = { key: string; file: FileEntry; fileIndex: number };

export type Row = Base &
  (
    | { kind: 'header'; collapsed: boolean }
    | {
        kind: 'hunk';
        /** `@@ -a,b +c,d @@`, on the row right above a hunk. */
        header: string | null;
        /** The hidden lines this row can reveal. */
        gap: Gap | null;
        directions: ExpandDirection[];
      }
    | { kind: 'line'; line: InlineLine }
    | { kind: 'split'; left: Cell | null; right: Cell | null }
    | { kind: 'message'; message: Message }
    /** A file whose diff isn't loaded yet, sized for about `lines` lines. */
    | { kind: 'placeholder'; lines: number }
    /** The bottom of the file's card, and the space before the next one. */
    | { kind: 'end'; closed: boolean }
  );

export type RowOptions = {
  style: DiffStyle;
  diffs: (path: string) => LoadedDiff | 'failed' | undefined;
  collapsed: ReadonlySet<string>;
  /** Context lines (PRD §6.8), for estimating unloaded files. */
  context: number;
};

/** `@@ -a,b +c,d @@`, leaving out a count of 1 as git does. */
export function hunkHeader(hunk: Hunk): string {
  const range = (start: number, lines: number) => (lines === 1 ? `${start}` : `${start},${lines}`);
  return `@@ -${range(hunk.oldStart, hunk.oldLines)} +${range(hunk.newStart, hunk.newLines)} @@`;
}

/** A run of a highlighted line, up to UTF-16 offset `end`; `style` is inline CSS. */
export type Token = { end: number; style: string | null };

export type Span = {
  text: string;
  changed: boolean;
  style: string | null;
  /** A match of find in page, and whether it's the current one. */
  mark: 'match' | 'current' | null;
};

/**
 * Splits `text` into runs, marking the changed `[start, end)` ranges and the
 * matches of find in page, and styling the runs that syntax highlighting
 * covers. Highlighting that doesn't fit the text (it changed, or was split
 * differently) is left out.
 */
export function spans(
  text: string,
  words: [number, number][] | null,
  tokens: Token[] | undefined,
  marks: readonly Mark[] = [],
): Span[] {
  const runs = tokens && tokens.length > 0 && tokens[tokens.length - 1].end <= text.length;
  const styled = runs ? tokens : [];
  // Every offset where a word range, a token or a mark starts or ends.
  const cuts = new Set([0, text.length]);
  for (const [start, end] of words ?? []) cuts.add(start).add(end);
  for (const { end } of styled) cuts.add(end);
  for (const { start, end } of marks) cuts.add(start).add(end);
  const offsets = [...cuts].filter((at) => at <= text.length).sort((a, b) => a - b);

  const parts: Span[] = [];
  let word = 0;
  let token = 0;
  let mark = 0;
  for (let i = 0; i + 1 < offsets.length; i++) {
    const [start, end] = [offsets[i], offsets[i + 1]];
    while (words && word < words.length && words[word][1] <= start) word++;
    while (token < styled.length && styled[token].end <= start) token++;
    while (mark < marks.length && marks[mark].end <= start) mark++;
    const changed = !!words && word < words.length && words[word][0] <= start;
    const style = styled[token]?.style ?? null;
    const marked = mark < marks.length && marks[mark].start <= start ? marks[mark] : null;
    const kind = marked ? (marked.current ? 'current' : 'match') : null;
    const last = parts[parts.length - 1];
    // Each mark stays one run of its own, so it can be found in the page.
    if (last && last.changed === changed && last.style === style && last.mark === kind && !kind)
      last.text += text.slice(start, end);
    else parts.push({ text: text.slice(start, end), changed, style, mark: kind });
  }
  return parts.length > 0 ? parts : [{ text, changed: false, style: null, mark: null }];
}

/** The lines a file's diff is expected to take before it's loaded. */
export function estimateLines(file: FileEntry, style: DiffStyle, context: number): number {
  const changed = file.additions + file.deletions;
  if (file.binary || changed > LARGE_DIFF_LINES) return 2;
  const lines = style === 'split' ? Math.max(file.additions, file.deletions) : changed;
  return Math.max(1, lines + 2 * context + 1);
}

/** No lines revealed. */
export const NO_CONTEXT: ReadonlyMap<number, string> = new Map();

/** `context` with `lines` revealed from new-side line `start` on. */
export function revealLines(
  context: ReadonlyMap<number, string>,
  start: number,
  lines: string[],
): ReadonlyMap<number, string> {
  return new Map([...context, ...lines.map((text, i) => [start + i, text] as const)]);
}

/**
 * The runs of `lines` (new-side line numbers revealed in an earlier version of
 * a file) that are still hidden unchanged lines in `diff`, as `[start, end]`
 * ranges. Used to reveal them again after a refresh (PRD §6.9).
 */
export function hiddenRuns(lines: Iterable<number>, diff: FileDiff): [number, number][] {
  if (diff.kind !== 'text') return [];
  const shown = (no: number) =>
    diff.hunks.some((hunk) => no >= hunk.newStart && no < hunk.newStart + hunk.newLines);
  const runs: [number, number][] = [];
  for (const no of [...lines].sort((a, b) => a - b)) {
    if (no < 1 || no > diff.newLineCount || shown(no)) continue;
    const last = runs.at(-1);
    if (last && last[1] === no - 1) last[1] = no;
    else runs.push([no, no]);
  }
  return runs;
}

/** The hidden lines that expanding `gap` towards `direction` reveals. */
export function expandRange(gap: Gap, direction: ExpandDirection): [number, number] {
  switch (direction) {
    case 'up':
      return [Math.max(gap.start, gap.end - EXPAND_STEP + 1), gap.end];
    case 'down':
      return [gap.start, Math.min(gap.end, gap.start + EXPAND_STEP - 1)];
    case 'all':
      return [gap.start, gap.end];
  }
}

export function buildRows(files: FileEntry[], options: RowOptions): Row[] {
  const rows: Row[] = [];
  files.forEach((file, fileIndex) => {
    const push = (key: string, row: DistributiveOmit<Row, keyof Base>) =>
      rows.push({ ...row, key: `${file.path}\0${key}`, file, fileIndex } as Row);
    const collapsed = options.collapsed.has(file.path);
    push('f', { kind: 'header', collapsed });
    if (!collapsed) fileBody(file, options, push);
    push('e', { kind: 'end', closed: !collapsed });
  });
  return rows;
}

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;
type Push = (key: string, row: DistributiveOmit<Row, keyof Base>) => void;

function fileBody(file: FileEntry, options: RowOptions, push: Push) {
  if (file.submodule) {
    const [from, to] = file.submodule;
    push('m', { kind: 'message', message: { kind: 'submodule', from, to } });
    return;
  }
  const loaded = options.diffs(file.path);
  if (loaded === undefined) {
    push('p', { kind: 'placeholder', lines: estimateLines(file, options.style, options.context) });
    return;
  }
  if (loaded === 'failed') {
    push('m', { kind: 'message', message: { kind: 'failed', text: "Couldn't load this diff." } });
    return;
  }
  const { diff } = loaded;
  switch (diff.kind) {
    case 'binary':
      push('m', { kind: 'message', message: { ...diff, kind: 'binary' } });
      return;
    case 'tooLarge':
    case 'whitespaceOnly':
      push('m', { kind: 'message', message: { kind: diff.kind } });
      return;
  }
  if (file.modeChange) {
    const [from, to] = file.modeChange;
    push('mode', { kind: 'message', message: { kind: 'modeChange', from, to } });
  }
  if (diff.hunks.length === 0) {
    if (!file.modeChange) {
      push('m', { kind: 'message', message: { kind: 'noContent', text: noContent(file) } });
    }
    return;
  }
  textRows(diff, loaded.context, options.style, push);
}

function noContent(file: FileEntry): string {
  switch (file.status) {
    case 'R':
      return 'File renamed without changes.';
    case 'A':
      return 'Empty file added.';
    case 'D':
      return 'Empty file deleted.';
    default:
      return 'No changes to the content.';
  }
}

type TextDiff = Extract<FileDiff, { kind: 'text' }>;

function textRows(
  diff: TextDiff,
  context: ReadonlyMap<number, string>,
  style: DiffStyle,
  push: Push,
) {
  const eof = (kind: LineKind, oldNo: number | null, newNo: number | null) => ({
    old: kind !== 'add' && diff.noNewlineAtEof.old && oldNo === diff.oldLineCount,
    new: kind !== 'del' && diff.noNewlineAtEof.new && newNo === diff.newLineCount,
  });
  const revealed = [...context.keys()].sort((a, b) => a - b);

  const contextLine = (newNo: number, oldNo: number) => {
    const text = context.get(newNo)!;
    const end = eof('ctx', oldNo, newNo);
    if (style === 'split') {
      const cell = (no: number, noNewline: boolean): Cell => ({
        kind: 'ctx',
        no,
        text,
        words: null,
        noNewline,
      });
      push(`s${oldNo}:${newNo}`, {
        kind: 'split',
        left: cell(oldNo, end.old),
        right: cell(newNo, end.new),
      });
    } else {
      const line = { kind: 'ctx' as const, oldNo, newNo, text, words: null };
      push(`l${oldNo}:${newNo}`, {
        kind: 'line',
        line: { ...line, noNewline: end.old || end.new },
      });
    }
  };

  /** The unchanged lines of `gap`: revealed ones, and controls for the hidden runs. */
  const gapRows = (gap: Gap, header: string | null) => {
    const hidden = (start: number, end: number) => {
      const run = { start, end, oldOffset: gap.oldOffset };
      const directions: ExpandDirection[] =
        end - start + 1 <= EXPAND_STEP
          ? ['all']
          : [
              ...(start > 1 ? (['down'] as const) : []),
              ...(end < diff.newLineCount ? (['up'] as const) : []),
            ];
      push(`h${end + 1}`, {
        kind: 'hunk',
        header: end === gap.end ? header : null,
        gap: run,
        directions,
      });
    };
    let next = gap.start;
    for (const no of revealed) {
      if (no < gap.start || no > gap.end) continue;
      if (no > next) hidden(next, no - 1);
      contextLine(no, no + gap.oldOffset);
      next = no + 1;
    }
    if (next <= gap.end) hidden(next, gap.end);
    else if (header !== null && gap.start > gap.end) {
      // The first hunk starts at the top of the file: nothing to expand.
      push(`h${gap.start}`, { kind: 'hunk', header, gap: null, directions: [] });
    }
  };

  let prevOld = 0;
  let prevNew = 0;
  for (const hunk of diff.hunks) {
    // A side with no lines starts at the line before the change.
    const oldFirst = hunk.oldLines > 0 ? hunk.oldStart : hunk.oldStart + 1;
    const newFirst = hunk.newLines > 0 ? hunk.newStart : hunk.newStart + 1;
    gapRows(
      { start: prevNew + 1, end: newFirst - 1, oldOffset: oldFirst - newFirst },
      hunkHeader(hunk),
    );
    hunkLines(hunk.lines, style, eof, push);
    prevOld = oldFirst + hunk.oldLines - 1;
    prevNew = newFirst + hunk.newLines - 1;
  }
  gapRows({ start: prevNew + 1, end: diff.newLineCount, oldOffset: prevOld - prevNew }, null);
}

type Eof = (
  kind: LineKind,
  oldNo: number | null,
  newNo: number | null,
) => { old: boolean; new: boolean };

function hunkLines(lines: Line[], style: DiffStyle, eof: Eof, push: Push) {
  if (style === 'inline') {
    for (const line of lines) {
      const end = eof(line.kind, line.oldNo, line.newNo);
      push(`l${line.oldNo ?? ''}:${line.newNo ?? ''}`, {
        kind: 'line',
        line: { ...line, noNewline: end.old || end.new },
      });
    }
    return;
  }

  const cell = (line: Line, side: 'old' | 'new'): Cell => ({
    kind: line.kind,
    no: (side === 'old' ? line.oldNo : line.newNo)!,
    text: line.text,
    words: line.words,
    noNewline: eof(line.kind, line.oldNo, line.newNo)[side],
  });
  const splitRow = (left: Cell | null, right: Cell | null) =>
    push(`s${left?.no ?? ''}:${right?.no ?? ''}`, { kind: 'split', left, right });

  // Within a changed block, deletions go left and additions right, paired in
  // order, with filler cells on the shorter side.
  let deleted: Line[] = [];
  let added: Line[] = [];
  const flush = () => {
    for (let i = 0; i < Math.max(deleted.length, added.length); i++) {
      splitRow(
        deleted[i] ? cell(deleted[i], 'old') : null,
        added[i] ? cell(added[i], 'new') : null,
      );
    }
    deleted = [];
    added = [];
  };
  for (const line of lines) {
    if (line.kind === 'del') deleted.push(line);
    else if (line.kind === 'add') added.push(line);
    else {
      flush();
      splitRow(cell(line, 'old'), cell(line, 'new'));
    }
  }
  flush();
}
