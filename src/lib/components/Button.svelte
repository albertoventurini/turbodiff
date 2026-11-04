<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  let {
    variant = 'default',
    size = 'medium',
    children,
    ...rest
  }: HTMLButtonAttributes & {
    variant?: 'default' | 'primary' | 'invisible';
    size?: 'medium' | 'small';
    children: Snippet;
  } = $props();
</script>

<button type="button" class="button {variant} {size}" {...rest}>
  {@render children()}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--control-medium-gap);
    height: var(--control-medium-size);
    padding: 0 var(--control-medium-paddingInline-normal);
    border: var(--borderWidth-thin) solid;
    border-radius: var(--borderRadius-medium);
    font-size: var(--text-body-size-medium);
    font-weight: var(--base-text-weight-medium);
    white-space: nowrap;
    cursor: pointer;
  }

  .small {
    gap: var(--control-small-gap);
    height: var(--control-small-size);
    padding: 0 var(--control-small-paddingInline-condensed);
    font-size: var(--text-body-size-small);
  }

  .button:disabled {
    cursor: not-allowed;
  }

  .default {
    background: var(--button-default-bgColor-rest);
    border-color: var(--button-default-borderColor-rest);
    color: var(--button-default-fgColor-rest);
    box-shadow: var(--button-default-shadow-resting);
  }
  .default:hover:not(:disabled) {
    background: var(--button-default-bgColor-hover);
    border-color: var(--button-default-borderColor-hover);
  }
  .default:active:not(:disabled) {
    background: var(--button-default-bgColor-active);
    border-color: var(--button-default-borderColor-active);
  }
  .default:disabled {
    background: var(--button-default-bgColor-disabled);
    border-color: var(--button-default-borderColor-disabled);
    color: var(--button-default-fgColor-disabled);
  }

  .primary {
    background: var(--button-primary-bgColor-rest);
    border-color: var(--button-primary-borderColor-rest);
    color: var(--button-primary-fgColor-rest);
  }
  .primary:hover:not(:disabled) {
    background: var(--button-primary-bgColor-hover);
    border-color: var(--button-primary-borderColor-hover);
  }
  .primary:active:not(:disabled) {
    background: var(--button-primary-bgColor-active);
    border-color: var(--button-primary-borderColor-active);
  }

  .invisible {
    background: var(--button-invisible-bgColor-rest);
    border-color: var(--button-invisible-borderColor-rest);
    color: var(--button-invisible-fgColor-rest);
  }
  .invisible :global(.icon) {
    color: var(--button-invisible-iconColor-rest);
  }
  .invisible:hover:not(:disabled) {
    background: var(--button-invisible-bgColor-hover);
    border-color: var(--button-invisible-borderColor-hover);
  }
  .invisible:hover:not(:disabled) :global(.icon) {
    color: var(--button-invisible-iconColor-hover);
  }
  .invisible:active:not(:disabled) {
    background: var(--button-invisible-bgColor-active);
  }
  .invisible:disabled,
  .invisible:disabled :global(.icon) {
    color: var(--fgColor-disabled);
  }
</style>
