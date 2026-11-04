import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { OUTPUT, render } from '../../scripts/build-tokens.js';

const SRC = join(import.meta.dirname, '..');

/** The hand-written styles: components and stylesheets, without generated files. */
function styleSources(): { path: string; text: string }[] {
  return readdirSync(SRC, { recursive: true, encoding: 'utf8' })
    .filter((path) => /\.(svelte|css)$/.test(path) && !path.endsWith('themes.css'))
    .map((path) => ({ path, text: readFileSync(join(SRC, path), 'utf8') }));
}

describe('theme tokens', () => {
  it('match the installed @primer/primitives', () => {
    // Run `pnpm build:tokens` if this fails.
    expect(readFileSync(OUTPUT, 'utf8')).toBe(render());
  });

  it('define the same tokens in light and dark', () => {
    const blocks = render().split('\n[data-theme=').slice(1);
    const names = blocks.map((block) => block.match(/--[\w-]+/g)!.join());
    expect(blocks).toHaveLength(2);
    expect(names[0]).toBe(names[1]);
  });

  it('are the only colours in the components', () => {
    const raw = /#[0-9a-f]{3,8}\b|\b(rgba?|hsla?|oklch|color-mix)\(/i;
    for (const { path, text } of styleSources()) {
      expect(text.match(raw), path).toBeNull();
    }
  });

  it('cover every custom property the components use', () => {
    // The mode-independent Primer tokens that global.css imports.
    const primer = [
      ...readFileSync(join(SRC, 'styles/global.css'), 'utf8').matchAll(
        /@import '(@primer\/[^']+)'/g,
      ),
    ].map((m) => readFileSync(join(SRC, '../node_modules', m[1]), 'utf8'));
    expect(primer.length).toBeGreaterThan(0);
    const defined = new Set(
      [render(), ...primer, ...styleSources().map((source) => source.text)].flatMap((css) =>
        [...css.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]),
      ),
    );
    for (const { path, text } of styleSources()) {
      for (const [, name] of text.matchAll(/var\((--[\w-]+)/g)) {
        expect(defined.has(name), `${path} uses ${name}`).toBe(true);
      }
    }
  });
});
