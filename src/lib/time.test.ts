import { describe, expect, it } from 'vitest';
import { relativeTime } from './time';

describe('relativeTime', () => {
  const now = Date.UTC(2026, 10, 4);
  const ago = (seconds: number) => relativeTime(now / 1000 - seconds, now);

  it('picks the largest whole unit', () => {
    expect(ago(10)).toBe('just now');
    expect(ago(5 * 60)).toBe('5 minutes ago');
    expect(ago(3 * 3600)).toBe('3 hours ago');
    expect(ago(30 * 3600)).toBe('yesterday');
    expect(ago(10 * 24 * 3600)).toBe('last week');
    expect(ago(400 * 24 * 3600)).toBe('last year');
  });
});
