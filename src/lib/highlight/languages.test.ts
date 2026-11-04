import { describe, expect, it } from 'vitest';
import { LANGUAGES, languageFor } from './languages';

describe('languageFor', () => {
  it('goes by extension, ignoring case', () => {
    expect(languageFor('src/main.rs')).toBe('rust');
    expect(languageFor('src/App.svelte')).toBe('svelte');
    expect(languageFor('README.MD')).toBe('markdown');
    expect(languageFor('a/b.c/types.d.ts')).toBe('typescript');
    expect(languageFor('.eslintrc.json')).toBe('json');
  });

  it('knows some files by name', () => {
    expect(languageFor('Dockerfile')).toBe('docker');
    expect(languageFor('build/Makefile')).toBe('make');
    expect(languageFor('CMakeLists.txt')).toBe('cmake');
    expect(languageFor('Cargo.lock')).toBe('toml');
    expect(languageFor('home/.bashrc')).toBe('shellscript');
  });

  it('leaves the rest plain', () => {
    expect(languageFor('notes.txt')).toBeNull();
    expect(languageFor('LICENSE')).toBeNull();
    expect(languageFor('.hidden')).toBeNull();
    expect(languageFor('a.constructor')).toBeNull();
  });

  it('loads every grammar', async () => {
    for (const [id, load] of Object.entries(LANGUAGES)) {
      const grammars = (await load()).default;
      expect(
        grammars.some((grammar) => grammar.name === id),
        id,
      ).toBe(true);
    }
  });
});
