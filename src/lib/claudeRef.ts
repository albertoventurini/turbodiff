// "Copy for Claude" (PRD §6.6): a selection in the diff, as text to paste into
// a coding agent: where the code is, in which comparison, and the code itself.
import { languageFor } from './highlight/languages';
import { refLabel } from './refs';
import type { Comparison, FileEntry, LineKind } from './types';

/** The selected part of one line. Split cells have only their own side's number. */
export type SelectedLine = {
  kind: LineKind;
  oldNo: number | null;
  newNo: number | null;
  text: string;
};

export type SelectedFile = { file: FileEntry; lines: SelectedLine[] };

/** `HEAD → working tree`, `main...feature` or `main..feature`. */
export function comparisonLabel(comparison: Comparison, noMergeBase = false): string {
  const base = refLabel(comparison.base);
  if (comparison.compare.kind === 'workTree') return `${base} → working tree`;
  const dots = comparison.mode === 'threeDot' && !noMergeBase ? '...' : '..';
  return `${base}${dots}${refLabel(comparison.compare.spec)}`;
}

/** `42`, or `42-51` for several lines. */
function lineRange(numbers: number[]): string {
  const [first, last] = [Math.min(...numbers), Math.max(...numbers)];
  return first === last ? `${first}` : `${first}-${last}`;
}

/**
 * `path:42-51`, on the new side. Lines only in the old version (deleted ones,
 * or the left side of the split view) are numbered in the old version.
 */
function location({ file, lines }: SelectedFile): string {
  const newNos = lines.flatMap((line) => (line.newNo === null ? [] : [line.newNo]));
  const oldNos = lines.flatMap((line) =>
    line.newNo === null && line.oldNo !== null ? [line.oldNo] : [],
  );
  const old = oldNos.length > 0 ? `${file.oldPath ?? file.path}:${lineRange(oldNos)}` : null;
  if (newNos.length === 0) return `${old} (old version)`;
  const current = `${file.path}:${lineRange(newNos)}`;
  return old ? `${current} (old version: ${old})` : current;
}

const MARKERS: Record<LineKind, string> = { ctx: ' ', add: '+', del: '-' };

function block(selected: SelectedFile, comparison: string): string {
  const { file, lines } = selected;
  const changed = lines.some((line) => line.kind !== 'ctx');
  const body = lines
    .map((line) => (changed ? MARKERS[line.kind] + line.text : line.text))
    .join('\n');
  // A fence longer than any run of backticks in the code.
  const ticks = Math.max(2, ...[...body.matchAll(/`+/g)].map((run) => run[0].length)) + 1;
  const fence = '`'.repeat(ticks);
  const language = changed ? 'diff' : (languageFor(file.path) ?? '');
  return `${location(selected)}, ${comparison}:\n${fence}${language}\n${body}\n${fence}`;
}

/** The text that "Copy for Claude" puts on the clipboard, one block per file. */
export function claudeReference(
  files: SelectedFile[],
  comparison: Comparison,
  noMergeBase = false,
): string {
  const label = comparisonLabel(comparison, noMergeBase);
  return files
    .filter((selected) => selected.lines.length > 0)
    .map((selected) => block(selected, label))
    .join('\n\n');
}
