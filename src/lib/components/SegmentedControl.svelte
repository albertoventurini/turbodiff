<script lang="ts" generics="T extends string">
  import type { Snippet } from 'svelte';

  type Option = { value: T; label: string; title?: string };

  let {
    options,
    value = $bindable(),
    label,
    title,
    onchange,
    item,
  }: {
    options: Option[];
    value: T;
    /** The accessible name of the group. */
    label: string;
    /** A tooltip for the whole control. */
    title?: string;
    onchange?: (value: T) => void;
    /** Renders an option's content, instead of its label. */
    item?: Snippet<[Option]>;
  } = $props();

  function choose(option: T) {
    if (option === value) return;
    value = option;
    onchange?.(option);
  }
</script>

<div class="segmented" role="group" aria-label={label} {title}>
  {#each options as option (option.value)}
    <button
      type="button"
      class="segment"
      aria-pressed={option.value === value}
      aria-label={item ? option.label : undefined}
      title={option.title}
      onclick={() => choose(option.value)}
    >
      {#if item}{@render item(option)}{:else}{option.label}{/if}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    flex: none;
    height: var(--control-small-size);
    padding: var(--base-size-2);
    border: var(--borderWidth-thin) solid var(--controlTrack-borderColor-rest);
    border-radius: var(--borderRadius-medium);
    background: var(--controlTrack-bgColor-rest);
  }

  .segment {
    display: inline-flex;
    align-items: center;
    padding: 0 var(--control-xsmall-paddingInline-normal);
    border: var(--borderWidth-thin) solid transparent;
    border-radius: var(--borderRadius-small);
    background: none;
    color: var(--controlTrack-fgColor-rest);
    font-size: var(--text-body-size-small);
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
  }

  .segment:hover:not([aria-pressed='true']) {
    background: var(--controlTrack-bgColor-hover);
  }

  .segment[aria-pressed='true'] {
    border-color: var(--controlKnob-borderColor-rest);
    background: var(--controlKnob-bgColor-rest);
    color: var(--fgColor-default);
    font-weight: var(--base-text-weight-semibold);
  }
</style>
