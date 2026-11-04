<script lang="ts">
  import { getStores } from '../stores/context';

  const { session } = getStores();
</script>

<!-- The summary line and file tree come in M4 (PRD §6.5). -->
<div class="sidebar-content" data-testid="file-tree-placeholder">
  {#if session.changes}
    <p>
      {session.changes.totals.files} changed {session.changes.totals.files === 1 ? 'file' : 'files'}
    </p>
  {/if}
</div>

<style>
  .sidebar-content {
    padding: var(--base-size-12) var(--base-size-16);
    color: var(--fgColor-muted);
    font-size: var(--text-body-size-small);
  }

  p {
    margin: 0;
  }
</style>
