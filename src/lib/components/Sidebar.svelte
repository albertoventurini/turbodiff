<script lang="ts">
  import { getStores } from '../stores/context';
  import type { FileTreeMode } from '../stores/view.svelte';
  import FileTree from './FileTree.svelte';
  import SegmentedControl from './SegmentedControl.svelte';

  const { session, view } = getStores();

  const MODES: { value: FileTreeMode; label: string }[] = [
    { value: 'tree', label: 'Tree' },
    { value: 'flat', label: 'Flat' },
  ];
</script>

{#if session.changes}
  {@const { files, additions, deletions } = session.changes.totals}
  <div class="header">
    <p class="summary" data-testid="summary">
      <span class="files">{files} {files === 1 ? 'file' : 'files'} changed</span>
      <span class="additions">+{additions}</span>
      <span class="deletions">−{deletions}</span>
    </p>
    {#if session.changes.files.length > 0}
      <SegmentedControl options={MODES} bind:value={view.fileTreeMode} label="File list" />
    {/if}
  </div>
  <FileTree files={session.changes.files} />
{/if}

<style>
  .header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--base-size-8);
    padding: var(--base-size-12) var(--base-size-16) var(--base-size-8);
  }

  .summary {
    display: flex;
    gap: var(--base-size-8);
    margin: 0;
    font-size: var(--text-body-size-small);
    white-space: nowrap;
  }

  .files {
    font-weight: var(--base-text-weight-semibold);
  }

  .additions,
  .deletions {
    font-family: var(--fontStack-monospace);
  }

  .additions {
    color: var(--fgColor-success);
  }

  .deletions {
    color: var(--fgColor-danger);
  }
</style>
