<script lang="ts">
  import type { FileEntry } from '../types';
  import Icon from './Icon.svelte';
  import StatusBadge from './StatusBadge.svelte';

  let { file, collapsed, ontoggle }: { file: FileEntry; collapsed: boolean; ontoggle: () => void } =
    $props();

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copyPath() {
    try {
      await navigator.clipboard.writeText(file.path);
    } catch {
      return;
    }
    copied = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="file-header" class:collapsed>
  <button
    type="button"
    class="icon-button"
    aria-expanded={!collapsed}
    aria-label={collapsed ? 'Expand file' : 'Collapse file'}
    title={collapsed ? 'Expand file' : 'Collapse file'}
    onclick={ontoggle}
  >
    <Icon name={collapsed ? 'chevron-right' : 'chevron-down'} />
  </button>
  <StatusBadge status={file.status} />
  <span class="path" title={file.oldPath ? `${file.oldPath} → ${file.path}` : file.path}>
    {#if file.oldPath}
      <span class="old">{file.oldPath}</span>
      <span class="arrow" aria-label="renamed to">→</span>
    {/if}
    <span>{file.path}</span>
  </span>
  {#if file.similarity !== null && file.status === 'R'}
    <span class="similarity">{file.similarity}% similar</span>
  {/if}
  <button
    type="button"
    class="icon-button"
    class:copied
    aria-label="Copy path"
    title={copied ? 'Copied' : 'Copy path'}
    onclick={copyPath}
  >
    <Icon name={copied ? 'check' : 'copy'} />
  </button>
  <span class="counts">
    {#if file.binary}
      <span class="bin">bin</span>
    {:else}
      <span class="additions">+{file.additions}</span>
      <span class="deletions">−{file.deletions}</span>
    {/if}
  </span>
</div>

<style>
  .file-header {
    display: flex;
    align-items: center;
    gap: var(--base-size-8);
    height: 100%;
    padding: 0 var(--base-size-8);
    border: var(--borderWidth-thin) solid var(--borderColor-default);
    border-radius: var(--borderRadius-medium) var(--borderRadius-medium) 0 0;
    background: var(--bgColor-muted);
    font-size: var(--text-body-size-small);
    user-select: none;
  }

  .collapsed {
    border-radius: var(--borderRadius-medium);
  }

  .icon-button {
    display: flex;
    flex: none;
    padding: var(--base-size-4);
    border: 0;
    border-radius: var(--borderRadius-medium);
    background: none;
    color: var(--fgColor-muted);
    cursor: pointer;
  }

  .icon-button:hover {
    background: var(--button-invisible-bgColor-hover);
    color: var(--fgColor-default);
  }

  .copied,
  .copied:hover {
    color: var(--fgColor-success);
  }

  .path {
    display: flex;
    gap: var(--base-size-4);
    min-width: 0;
    overflow: hidden;
    font-family: var(--fontStack-monospace);
    font-weight: var(--base-text-weight-semibold);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .path > span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .old {
    flex: 0 1 auto;
    color: var(--fgColor-muted);
    font-weight: var(--base-text-weight-normal);
  }

  .arrow {
    flex: none;
    color: var(--fgColor-muted);
  }

  .similarity {
    flex: none;
    color: var(--fgColor-muted);
  }

  .counts {
    display: flex;
    flex: none;
    gap: var(--base-size-4);
    margin-left: auto;
    font-family: var(--fontStack-monospace);
    font-size: var(--text-caption-size);
  }

  .additions {
    color: var(--fgColor-success);
  }
  .deletions {
    color: var(--fgColor-danger);
  }
  .bin {
    color: var(--fgColor-muted);
  }
</style>
