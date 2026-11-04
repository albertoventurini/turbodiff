// How refs and comparisons are named in the UI.
import type { CompareSide } from './types';

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
