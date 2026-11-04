<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    tone = 'attention',
    children,
    ondismiss,
  }: {
    tone?: 'attention' | 'danger';
    children: Snippet;
    ondismiss?: () => void;
  } = $props();
</script>

<div class="notice {tone}" role={tone === 'danger' ? 'alert' : 'status'}>
  <Icon name={tone === 'danger' ? 'stop' : 'alert'} />
  <div class="text">{@render children()}</div>
  {#if ondismiss}
    <button type="button" class="dismiss" aria-label="Dismiss" onclick={ondismiss}>
      <Icon name="x" />
    </button>
  {/if}
</div>

<style>
  .notice {
    display: flex;
    align-items: flex-start;
    gap: var(--base-size-8);
    padding: var(--base-size-8) var(--base-size-16);
    border: var(--borderWidth-thin) solid;
    border-radius: var(--borderRadius-medium);
  }

  .notice > :global(.icon) {
    margin-top: var(--base-size-2);
  }

  .attention {
    background: var(--bgColor-attention-muted);
    border-color: var(--borderColor-attention-muted);
  }
  .attention > :global(.icon) {
    color: var(--fgColor-attention);
  }

  .danger {
    background: var(--bgColor-danger-muted);
    border-color: var(--borderColor-danger-muted);
  }
  .danger > :global(.icon) {
    color: var(--fgColor-danger);
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .text :global(code) {
    font-family: var(--fontStack-monospace);
    font-size: 85%;
  }

  .dismiss {
    display: flex;
    padding: var(--base-size-2);
    border: 0;
    border-radius: var(--borderRadius-small);
    background: none;
    color: var(--fgColor-muted);
    cursor: pointer;
  }

  .dismiss:hover {
    background: var(--button-invisible-bgColor-hover);
    color: var(--fgColor-default);
  }
</style>
