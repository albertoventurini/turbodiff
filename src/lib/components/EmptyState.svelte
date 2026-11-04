<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  let {
    icon,
    title,
    tone = 'default',
    children,
    actions,
  }: {
    icon: IconName;
    title: string;
    tone?: 'default' | 'danger';
    children?: Snippet;
    actions?: Snippet;
  } = $props();
</script>

<section class="empty {tone}" aria-label={title}>
  <span class="glyph"><Icon name={icon} size={24} /></span>
  <h2>{title}</h2>
  {#if children}
    <div class="description">{@render children()}</div>
  {/if}
  {#if actions}
    <div class="actions">{@render actions()}</div>
  {/if}
</section>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    max-width: 32rem;
    margin: 0 auto;
    padding: var(--base-size-40) var(--base-size-24);
    text-align: center;
  }

  .glyph {
    display: flex;
    margin-bottom: var(--base-size-16);
    color: var(--fgColor-muted);
  }

  .danger .glyph {
    color: var(--fgColor-danger);
  }

  h2 {
    margin: 0 0 var(--base-size-4);
    font-size: var(--text-title-size-medium);
    font-weight: var(--text-title-weight-medium);
    line-height: var(--text-title-lineHeight-medium);
  }

  .description {
    color: var(--fgColor-muted);
    font-size: var(--text-body-size-large);
  }

  .description :global(p) {
    margin: 0 0 var(--base-size-8);
  }

  .description :global(code) {
    padding: var(--base-size-2) var(--base-size-4);
    border-radius: var(--borderRadius-medium);
    background: var(--bgColor-neutral-muted);
    font-family: var(--fontStack-monospace);
    font-size: 85%;
  }

  .actions {
    display: flex;
    gap: var(--base-size-8);
    margin-top: var(--base-size-16);
  }
</style>
