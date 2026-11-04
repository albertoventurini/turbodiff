// Shiki, set up with its JavaScript regex engine (no WASM), turning a whole
// file into each line's coloured runs (TECH.md §7.4). Runs in the highlight
// worker; it's plain code, so tests call it directly.
import { createHighlighterCore, type HighlighterCore, type ThemeRegistration } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';
import { LANGUAGES, type LanguageId } from './languages';

/** The Shiki themes that ship (see `THEMES` in theme.svelte.ts). */
const THEME_LOADERS: Record<string, () => Promise<{ default: ThemeRegistration }>> = {
  'github-light': () => import('shiki/themes/github-light.mjs'),
  'github-dark': () => import('shiki/themes/github-dark.mjs'),
};

/** Longer lines are left plain: tokenizing them is slow and rarely useful. */
export const MAX_LINE_LENGTH = 1000;
/** How long one line may take to tokenize, in milliseconds. */
const LINE_TIME_LIMIT = 500;

/**
 * A file's highlighting. Each line is a list of `[end, style]` runs, `end`
 * a UTF-16 offset in the line and `style` an index into `styles`, or -1 for
 * text in the theme's default colour, which is left to the diff's own.
 */
export type Tokens = { styles: string[]; lines: [number, number][][] };

// Shiki's FontStyle flags.
const ITALIC = 1;
const BOLD = 2;
const UNDERLINE = 4;
const STRIKETHROUGH = 8;

let highlighter: Promise<HighlighterCore> | null = null;
const loading = new Map<string, Promise<void>>();

function core(): Promise<HighlighterCore> {
  highlighter ??= createHighlighterCore({
    engine: createJavaScriptRegexEngine(),
    warnings: false,
  });
  return highlighter;
}

/** Loads a grammar or theme into the highlighter once. */
function once(key: string, load: () => Promise<void>): Promise<void> {
  let promise = loading.get(key);
  if (!promise) {
    promise = load();
    loading.set(key, promise);
  }
  return promise;
}

function css(color: string | undefined, fontStyle: number): string {
  const parts: string[] = [];
  if (color) parts.push(`color:${color}`);
  if (fontStyle > 0) {
    if (fontStyle & ITALIC) parts.push('font-style:italic');
    if (fontStyle & BOLD) parts.push('font-weight:bold');
    const lines = [
      fontStyle & UNDERLINE && 'underline',
      fontStyle & STRIKETHROUGH && 'line-through',
    ];
    if (lines.some(Boolean)) parts.push(`text-decoration:${lines.filter(Boolean).join(' ')}`);
  }
  return parts.join(';');
}

export async function tokenize(text: string, lang: LanguageId, theme: string): Promise<Tokens> {
  const shiki = await core();
  const loadTheme = THEME_LOADERS[theme];
  if (!loadTheme) throw new Error(`unknown theme ${theme}`);
  await Promise.all([
    once(`lang:${lang}`, async () => shiki.loadLanguage((await LANGUAGES[lang]()).default)),
    once(`theme:${theme}`, async () => shiki.loadTheme((await loadTheme()).default)),
  ]);

  const result = shiki.codeToTokens(text, {
    lang,
    theme,
    tokenizeMaxLineLength: MAX_LINE_LENGTH,
    tokenizeTimeLimit: LINE_TIME_LIMIT,
  });
  const fg = result.fg?.toLowerCase();
  const styles: string[] = [];
  const index = new Map<string, number>();
  const lines = result.tokens.map((tokens) => {
    const runs: [number, number][] = [];
    let end = 0;
    for (const token of tokens) {
      end += token.content.length;
      const color = token.color?.toLowerCase() === fg ? undefined : token.color;
      const style = css(color, token.fontStyle ?? 0);
      let id = -1;
      if (style) {
        id = index.get(style) ?? styles.push(style) - 1;
        index.set(style, id);
      }
      // Neighbouring runs in the same style become one.
      if (runs.length > 0 && runs[runs.length - 1][1] === id) runs[runs.length - 1][0] = end;
      else runs.push([end, id]);
    }
    return runs;
  });
  return { styles, lines };
}
