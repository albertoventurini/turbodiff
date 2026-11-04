<script lang="ts">
  import { onMount } from 'svelte';
  import * as api from '../api';
  import { getStores } from '../stores/context';
  import { CONTEXT_LINES, FONT_SIZES } from '../stores/settings.svelte';
  import type { Algorithm, Appearance, DiffStyle, SettingsPatch } from '../types';
  import Button from './Button.svelte';
  import SegmentedControl from './SegmentedControl.svelte';

  const { settings } = getStores();

  const current = $derived(settings.value);

  const APPEARANCES: { value: Appearance; label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ];
  const STYLES: { value: DiffStyle; label: string }[] = [
    { value: 'inline', label: 'Inline' },
    { value: 'split', label: 'Side-by-side' },
  ];
  const ALGORITHMS: { value: Algorithm; label: string }[] = [
    { value: 'histogram', label: 'Histogram' },
    { value: 'myers', label: 'Myers' },
  ];
  const WRAPPING = [
    { value: 'off', label: 'Off' },
    { value: 'on', label: 'On' },
  ] as const;
  const TAB_WIDTHS = [
    { value: '2', label: '2' },
    { value: '4', label: '4' },
    { value: '8', label: '8' },
  ] as const;

  /** The installed monospace fonts, or `null` until they're loaded. */
  let fonts = $state<string[] | null>(null);

  onMount(() => {
    api.listMonospaceFonts().then(
      (list) => (fonts = list),
      () => (fonts = []),
    );
  });

  /** A saved font that isn't installed (any more) stays selectable. */
  const missingFont = $derived(
    fonts !== null && current.codeFont !== '' && !fonts.includes(current.codeFont)
      ? current.codeFont
      : null,
  );

  function save(patch: SettingsPatch) {
    void settings.update(patch);
  }

  /** Saves a number input's value, clamped to its range. Text that isn't a
   * number puts the saved value back. */
  function saveNumber(
    input: HTMLInputElement,
    range: { min: number; max: number },
    patch: (value: number) => SettingsPatch,
    saved: number,
  ) {
    const parsed = Math.round(Number(input.value));
    const value =
      input.value.trim() === '' || !Number.isFinite(parsed)
        ? saved
        : Math.max(range.min, Math.min(range.max, parsed));
    input.value = String(value);
    if (value !== saved) save(patch(value));
  }
</script>

<main class="settings">
  <div class="content">
    <h1>Settings</h1>

    <section aria-labelledby="colours-heading">
      <h2 id="colours-heading">Colours</h2>
      <div class="row">
        <div class="label">
          <span id="appearance-label">Appearance</span>
          <p class="hint">System follows your operating system's light or dark mode.</p>
        </div>
        <SegmentedControl
          options={APPEARANCES}
          value={current.appearance}
          label="Appearance"
          onchange={(appearance) => save({ appearance })}
        />
      </div>
      <div class="row">
        <div class="label">
          <label for="theme">Theme</label>
        </div>
        <select id="theme" value={current.theme} disabled>
          <option value="github">GitHub</option>
        </select>
      </div>
    </section>

    <section aria-labelledby="diff-heading">
      <h2 id="diff-heading">Diff</h2>
      <div class="row">
        <div class="label">
          <span>Default diff style</span>
          <p class="hint">The toolbar switches style for the window you're in.</p>
        </div>
        <SegmentedControl
          options={STYLES}
          value={current.diffStyle}
          label="Default diff style"
          onchange={(diffStyle) => save({ diffStyle })}
        />
      </div>
      <div class="row">
        <div class="label">
          <span>Diff algorithm</span>
          <p class="hint">Histogram usually gives more readable diffs.</p>
        </div>
        <SegmentedControl
          options={ALGORITHMS}
          value={current.diffAlgorithm}
          label="Diff algorithm"
          onchange={(diffAlgorithm) => save({ diffAlgorithm })}
        />
      </div>
      <div class="row">
        <div class="label">
          <label for="context-lines">Context lines</label>
          <p class="hint">
            Unchanged lines shown around each change, {CONTEXT_LINES.min}–{CONTEXT_LINES.max}.
          </p>
        </div>
        <input
          id="context-lines"
          type="number"
          min={CONTEXT_LINES.min}
          max={CONTEXT_LINES.max}
          value={current.contextLines}
          onchange={(e) =>
            saveNumber(
              e.currentTarget,
              CONTEXT_LINES,
              (contextLines) => ({ contextLines }),
              current.contextLines,
            )}
        />
      </div>
      <div class="row">
        <div class="label">
          <span>Line wrapping</span>
          <p class="hint">Off scrolls long lines sideways.</p>
        </div>
        <SegmentedControl
          options={[...WRAPPING]}
          value={current.lineWrapping ? 'on' : 'off'}
          label="Line wrapping"
          onchange={(value) => save({ lineWrapping: value === 'on' })}
        />
      </div>
    </section>

    <section aria-labelledby="code-heading">
      <h2 id="code-heading">Code</h2>
      <div class="row">
        <div class="label">
          <label for="code-font">Code font</label>
        </div>
        <select
          id="code-font"
          value={current.codeFont}
          onchange={(e) => save({ codeFont: e.currentTarget.value })}
        >
          <option value="">Default monospace</option>
          {#if missingFont !== null}
            <option value={missingFont}>{missingFont} (not installed)</option>
          {:else if fonts === null && current.codeFont !== ''}
            <option value={current.codeFont}>{current.codeFont}</option>
          {/if}
          {#each fonts ?? [] as font (font)}
            <option value={font}>{font}</option>
          {/each}
        </select>
      </div>
      <div class="row">
        <div class="label">
          <label for="font-size">Font size</label>
          <p class="hint">In pixels, {FONT_SIZES.min}–{FONT_SIZES.max}.</p>
        </div>
        <input
          id="font-size"
          type="number"
          min={FONT_SIZES.min}
          max={FONT_SIZES.max}
          value={current.fontSize}
          onchange={(e) =>
            saveNumber(e.currentTarget, FONT_SIZES, (fontSize) => ({ fontSize }), current.fontSize)}
        />
      </div>
      <div class="row">
        <div class="label">
          <span>Tab width</span>
        </div>
        <SegmentedControl
          options={[...TAB_WIDTHS]}
          value={String(current.tabWidth) as '2' | '4' | '8'}
          label="Tab width"
          onchange={(value) => save({ tabWidth: Number(value) })}
        />
      </div>
    </section>

    <footer>
      <Button onclick={() => void settings.reset()}>Reset to defaults</Button>
    </footer>
  </div>
</main>

<style>
  .settings {
    height: 100%;
    overflow-y: auto;
  }

  .content {
    max-width: 640px;
    margin: 0 auto;
    padding: var(--base-size-24);
  }

  h1 {
    margin: 0 0 var(--base-size-8);
    font-size: var(--text-title-size-medium);
    font-weight: var(--text-title-weight-medium);
    line-height: var(--text-title-lineHeight-medium);
  }

  section {
    margin-top: var(--base-size-24);
  }

  h2 {
    margin: 0;
    padding-bottom: var(--base-size-8);
    border-bottom: var(--borderWidth-thin) solid var(--borderColor-muted);
    font-size: var(--text-body-size-large);
    font-weight: var(--base-text-weight-semibold);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--base-size-16);
    padding: var(--base-size-12) 0;
    border-bottom: var(--borderWidth-thin) solid var(--borderColor-muted);
  }

  .label {
    min-width: 0;
    font-weight: var(--base-text-weight-semibold);
  }

  .hint {
    margin: var(--base-size-2) 0 0;
    color: var(--fgColor-muted);
    font-size: var(--text-body-size-small);
    font-weight: var(--base-text-weight-normal);
  }

  select,
  input {
    flex: none;
    height: var(--control-small-size);
    padding: 0 var(--control-small-paddingInline-condensed);
    border: var(--borderWidth-thin) solid var(--control-borderColor-rest);
    border-radius: var(--borderRadius-medium);
    background: var(--control-bgColor-rest);
    color: var(--control-fgColor-rest);
    font: inherit;
    font-size: var(--text-body-size-small);
  }

  select {
    max-width: 50%;
  }

  select:disabled {
    border-color: var(--control-borderColor-disabled);
    background: var(--control-bgColor-disabled);
    color: var(--control-fgColor-disabled);
  }

  input {
    width: 5em;
  }

  footer {
    margin-top: var(--base-size-24);
  }
</style>
