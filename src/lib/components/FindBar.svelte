<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    query = $bindable(),
    count,
    current,
    searching = false,
    requests,
    onnext,
    onprevious,
    onclose,
  }: {
    query: string;
    count: number;
    /** The current match, from 0, or `null` when there's none. */
    current: number | null;
    /** Whether diffs are still loading, so more matches may come. */
    searching?: boolean;
    /** Counts `Mod+F` presses: each one focuses the input and selects its text. */
    requests: number;
    onnext: () => void;
    onprevious: () => void;
    onclose: () => void;
  } = $props();

  let input = $state<HTMLInputElement>();

  $effect(() => {
    void requests;
    input?.focus();
    input?.select();
  });

  const status = $derived.by(() => {
    if (query === '') return '';
    if (count === 0) return searching ? 'Searching…' : 'No results';
    return `${(current ?? 0) + 1} of ${count}${searching ? '+' : ''}`;
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) onprevious();
      else onnext();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

<div class="find-bar" role="search">
  <label class="field">
    <Icon name="search" />
    <input
      type="text"
      bind:this={input}
      bind:value={query}
      {onkeydown}
      placeholder="Find in diff"
      aria-label="Find in diff"
      autocomplete="off"
      spellcheck="false"
    />
    <span class="status" role="status">{status}</span>
  </label>
  <button
    type="button"
    aria-label="Previous match"
    title="Previous match"
    disabled={count === 0}
    onclick={onprevious}
  >
    <Icon name="chevron-up" />
  </button>
  <button
    type="button"
    aria-label="Next match"
    title="Next match"
    disabled={count === 0}
    onclick={onnext}
  >
    <Icon name="chevron-down" />
  </button>
  <button type="button" aria-label="Close" title="Close" onclick={onclose}>
    <Icon name="x" />
  </button>
</div>

<style>
  .find-bar {
    display: flex;
    align-items: center;
    gap: var(--base-size-4);
    padding: var(--base-size-4);
    border: var(--borderWidth-thin) solid var(--borderColor-default);
    border-radius: var(--borderRadius-medium);
    background: var(--overlay-bgColor);
    box-shadow: var(--shadow-floating-small);
  }

  .field {
    display: flex;
    align-items: center;
    gap: var(--base-size-8);
    width: 20rem;
    height: var(--control-small-size);
    padding: 0 var(--base-size-8);
    border: var(--borderWidth-thin) solid var(--control-borderColor-rest);
    border-radius: var(--borderRadius-medium);
    background: var(--bgColor-default);
    color: var(--fgColor-muted);
  }

  .field:focus-within {
    outline: var(--focus-outline);
    outline-offset: calc(-1 * var(--borderWidth-thin));
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--fgColor-default);
    font: inherit;
    font-size: var(--text-body-size-small);
  }

  input::placeholder {
    color: var(--fgColor-muted);
  }

  .status {
    flex: none;
    font-size: var(--text-body-size-small);
    font-variant-numeric: tabular-nums;
  }

  button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--control-small-size);
    height: var(--control-small-size);
    padding: 0;
    border: 0;
    border-radius: var(--borderRadius-medium);
    background: none;
    color: var(--fgColor-muted);
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    background: var(--control-transparent-bgColor-hover);
    color: var(--fgColor-default);
  }

  button:disabled {
    color: var(--fgColor-disabled);
    cursor: default;
  }
</style>
