// How refs and comparisons are named in the UI.
import type { CommitItem, CompareSide, HeadInfo, RefItem } from './types';

const PREFIXES = ['refs/heads/', 'refs/tags/', 'refs/remotes/'];

/** A short name for a ref or commit: `main` for `refs/heads/main`, 7 characters of a SHA. */
export function refLabel(spec: string): string {
  for (const prefix of PREFIXES) {
    if (spec.startsWith(prefix)) return spec.slice(prefix.length);
  }
  return /^[0-9a-f]{40}$/i.test(spec) ? spec.slice(0, 7) : spec;
}

export function compareLabel(side: CompareSide): string {
  return side.kind === 'workTree' ? 'Working tree' : refLabel(side.spec);
}

export type RefTab = 'branches' | 'tags' | 'commits';

/** Whether search text could be a SHA, full or abbreviated (TECH.md §4.2). */
export function looksLikeSha(text: string): boolean {
  return /^[0-9a-f]{4,40}$/i.test(text);
}

/** Whether `spec`, as kept in the comparison, names the branch or tag `item`. */
export function isSelectedRef(spec: string, item: RefItem): boolean {
  return spec === item.fullName || spec === item.name || (spec === 'HEAD' && item.current);
}

/** Whether `spec` names `commit`: its SHA, or `HEAD` while it's detached there. */
export function isSelectedCommit(spec: string, commit: CommitItem, head: HeadInfo): boolean {
  return spec === commit.sha || (spec === 'HEAD' && !head.branch && head.commit === commit.sha);
}

/**
 * The picker tab that holds `spec` (PRD §6.3). Refs from the picker are full
 * names or SHAs; the command line can give short names or any revision, which
 * are looked up in the branch and tag lists once they're loaded. Like git,
 * a tag wins over a branch of the same name.
 */
export function selectionTab(
  spec: string,
  head: HeadInfo,
  branches: RefItem[] | null,
  tags: RefItem[] | null,
): RefTab {
  if (spec.startsWith('refs/heads/') || spec.startsWith('refs/remotes/')) return 'branches';
  if (spec.startsWith('refs/tags/')) return 'tags';
  if (spec === 'HEAD') return head.branch ? 'branches' : 'commits';
  if (tags?.some((tag) => tag.name === spec)) return 'tags';
  if (branches?.some((branch) => branch.name === spec)) return 'branches';
  if (branches && tags) return 'commits';
  return looksLikeSha(spec) ? 'commits' : 'branches';
}
