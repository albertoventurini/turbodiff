// Text meets WCAG AA contrast in both modes (PRD §7). Many dark-mode tokens
// are translucent, so each background is composited over the ones beneath it.
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const themes = readFileSync(`${import.meta.dirname}/../styles/themes.css`, 'utf8');

type Rgba = [number, number, number, number];

/** Every mode's tokens, from the generated stylesheet. */
function modes(css: string): Map<string, Map<string, string>> {
  const result = new Map<string, Map<string, string>>();
  for (const [, mode, body] of css.matchAll(/\[data-mode='(\w+)'\] \{([^}]*)\}/g)) {
    const tokens = new Map<string, string>();
    for (const [, name, value] of body.matchAll(/--([\w-]+):\s*([^;]+);/g)) tokens.set(name, value);
    result.set(mode, tokens);
  }
  return result;
}

function parse(value: string): Rgba {
  const hex = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(value.trim());
  if (!hex) throw new Error(`not a hex colour: ${value}`);
  const n = parseInt(hex[1], 16);
  return [n >> 16, (n >> 8) & 255, n & 255, hex[2] ? parseInt(hex[2], 16) / 255 : 1];
}

/** `top` drawn over the opaque colour `below`. */
function over([r, g, b, a]: Rgba, below: Rgba): Rgba {
  return [r * a + below[0] * (1 - a), g * a + below[1] * (1 - a), b * a + below[2] * (1 - a), 1];
}

function luminance([r, g, b]: Rgba): number {
  const channel = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: Rgba, b: Rgba): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/**
 * Text colours and the backgrounds they're shown on, from the top one down;
 * `bgColor-default` is always the bottom.
 */
const PAIRS: [string, ...string[]][] = [
  ['fgColor-default', 'bgColor-default'],
  ['fgColor-muted', 'bgColor-default'],
  ['fgColor-muted', 'bgColor-muted'],
  ['fgColor-muted', 'overlay-bgColor'],
  ['fgColor-success', 'bgColor-default'],
  ['fgColor-danger', 'bgColor-default'],
  // Status badges.
  ['fgColor-success', 'bgColor-success-muted'],
  ['fgColor-attention', 'bgColor-attention-muted'],
  ['fgColor-danger', 'bgColor-danger-muted'],
  ['fgColor-accent', 'bgColor-accent-muted'],
  // Notices.
  ['fgColor-default', 'bgColor-attention-muted'],
  ['fgColor-default', 'bgColor-danger-muted'],
  // The diff.
  ['diffBlob-hunkLine-fgColor', 'diffBlob-hunkLine-bgColor'],
  ['diffBlob-additionNum-fgColor', 'diffBlob-additionNum-bgColor'],
  ['diffBlob-deletionNum-fgColor', 'diffBlob-deletionNum-bgColor'],
  ['diffBlob-additionLine-fgColor', 'diffBlob-additionLine-bgColor'],
  ['diffBlob-deletionLine-fgColor', 'diffBlob-deletionLine-bgColor'],
  [
    'diffBlob-additionWord-fgColor',
    'diffBlob-additionWord-bgColor',
    'diffBlob-additionLine-bgColor',
  ],
  [
    'diffBlob-deletionWord-fgColor',
    'diffBlob-deletionWord-bgColor',
    'diffBlob-deletionLine-bgColor',
  ],
  // Find in page: matches on any line, and the current match.
  ['diffBlob-additionLine-fgColor', 'bgColor-attention-muted', 'diffBlob-additionLine-bgColor'],
  ['diffBlob-deletionLine-fgColor', 'bgColor-attention-muted', 'diffBlob-deletionLine-bgColor'],
  ['fgColor-onEmphasis', 'bgColor-attention-emphasis'],
  // Controls.
  ['controlTrack-fgColor-rest', 'controlTrack-bgColor-rest'],
  ['button-default-fgColor-rest', 'button-default-bgColor-rest'],
  ['button-primary-fgColor-rest', 'button-primary-bgColor-rest'],
  ['control-fgColor-rest', 'bgColor-default'],
];

describe('contrast', () => {
  const all = modes(themes);

  it('finds both modes, and tells a failing pair', () => {
    expect([...all.keys()].sort()).toEqual(['dark', 'light']);
    const light = all.get('light')!;
    const white = parse(light.get('bgColor-default')!);
    expect(contrast(parse(light.get('fgColor-disabled')!), white)).toBeLessThan(4.5);
  });

  for (const [mode, tokens] of all) {
    const color = (name: string) => {
      const value = tokens.get(name);
      if (value === undefined) throw new Error(`no token --${name}`);
      return parse(value);
    };
    it.each(PAIRS)(`${mode}: %s on %s meets AA`, (fg, ...backgrounds) => {
      const bg = [...backgrounds, 'bgColor-default']
        .reverse()
        .reduce<Rgba | null>(
          (below, name) => (below ? over(color(name), below) : color(name)),
          null,
        )!;
      expect(contrast(over(color(fg), bg), bg)).toBeGreaterThanOrEqual(4.5);
    });
  }
});
