import { describe, expect, it } from 'vitest';
import { MAX_LINE_LENGTH, tokenize, type Tokens } from './tokenize';

/** The style of the run covering `offset` in line `line`. */
function styleAt(tokens: Tokens, line: number, offset: number): string | null {
  const run = tokens.lines[line].find(([end]) => end > offset)!;
  return run[1] === -1 ? null : tokens.styles[run[1]];
}

const code = ['const a = 1; /* one', 'still a comment', '*/ const b = "two";'].join('\n');

describe('tokenize', () => {
  it('gives each line runs that cover it', async () => {
    const tokens = await tokenize(code, 'typescript', 'github-light');
    expect(tokens.lines).toHaveLength(3);
    code.split('\n').forEach((line, i) => {
      const runs = tokens.lines[i];
      expect(runs.at(-1)![0]).toBe(line.length);
      // Ends only grow, and neighbours differ in style.
      runs.slice(1).forEach(([end, style], j) => {
        expect(end).toBeGreaterThan(runs[j][0]);
        expect(style).not.toBe(runs[j][1]);
      });
    });
  });

  it('highlights the whole file, so comments spanning lines are right', async () => {
    const tokens = await tokenize(code, 'typescript', 'github-light');
    const comment = styleAt(tokens, 0, 'const a = 1; /*'.length);
    expect(comment).toMatch(/^color:#/);
    expect(styleAt(tokens, 1, 3)).toBe(comment);
    expect(styleAt(tokens, 2, 0)).toBe(comment);
    expect(styleAt(tokens, 0, 0)).not.toBe(comment);
  });

  it("leaves text in the theme's own colour unstyled", async () => {
    const tokens = await tokenize('a + b', 'typescript', 'github-light');
    // The space after `a` is plain text.
    expect(styleAt(tokens, 0, 1)).toBeNull();
  });

  it('uses the colours of the requested theme', async () => {
    const light = await tokenize(code, 'typescript', 'github-light');
    const dark = await tokenize(code, 'typescript', 'github-dark');
    // `const` is red in light and pink in dark.
    expect(styleAt(light, 0, 0)).not.toBe(styleAt(dark, 0, 0));
  });

  it('leaves very long lines plain', async () => {
    const long = `const s = "${'x'.repeat(MAX_LINE_LENGTH)}";`;
    const tokens = await tokenize(`${long}\nconst t = 1;`, 'typescript', 'github-light');
    expect(tokens.lines[0]).toEqual([[long.length, -1]]);
    expect(styleAt(tokens, 1, 0)).not.toBeNull();
  });

  it('loads grammars that other grammars embed', async () => {
    const tokens = await tokenize(
      '<script lang="ts">\nlet a: number = 1;\n</script>',
      'svelte',
      'github-dark',
    );
    expect(styleAt(tokens, 1, 0)).not.toBeNull();
  });
});
