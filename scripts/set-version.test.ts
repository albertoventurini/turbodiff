import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { FILES, isVersion, setVersion } from './set-version.js';

const ROOT = `${import.meta.dirname}/..`;

describe('set-version', () => {
  it('finds the version in every file, and the same one in each', () => {
    const versions = FILES.map(({ path, pattern }) => {
      const text = readFileSync(`${ROOT}/${path}`, 'utf8');
      const updated = setVersion(text, pattern, '9.9.9');
      // Only the version line changes.
      expect(updated.split('\n').length).toBe(text.split('\n').length);
      expect(updated).not.toBe(text);
      return pattern.exec(text)![0];
    });
    const numbers = versions.map((match) => /(\d+\.\d+\.\d+[^"]*)"$/.exec(match)![1]);
    expect(new Set(numbers).size).toBe(1);
  });

  it('changes only the workspace version in Cargo.toml', () => {
    const cargo = FILES.find(({ path }) => path === 'Cargo.toml')!;
    const text = [
      '[workspace]',
      'members = ["a"]',
      '',
      '[workspace.package]',
      'edition = "2024"',
      'version = "0.1.0"',
      '',
      '[workspace.dependencies]',
      'serde = { version = "1.0.0" }',
    ].join('\n');
    const updated = setVersion(text, cargo.pattern, '0.2.0');
    expect(updated).toContain('version = "0.2.0"');
    expect(updated).toContain('serde = { version = "1.0.0" }');
  });

  it('takes SemVer versions only', () => {
    expect(isVersion('1.2.3')).toBe(true);
    expect(isVersion('1.2.3-rc.1')).toBe(true);
    expect(isVersion('1.2')).toBe(false);
    expect(isVersion('01.2.3')).toBe(false);
    expect(isVersion('v1.2.3')).toBe(false);
  });
});
