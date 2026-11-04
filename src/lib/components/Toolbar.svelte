<script lang="ts">
  import { compareLabel, refLabel } from '../refs';
  import { getStores } from '../stores/context';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  const { session, view } = getStores();
</script>

<header class="toolbar">
  <Button
    variant="invisible"
    size="small"
    aria-label={view.sidebarCollapsed ? 'Show sidebar' : 'Hide sidebar'}
    aria-expanded={!view.sidebarCollapsed}
    aria-controls="sidebar"
    title={view.sidebarCollapsed ? 'Show sidebar' : 'Hide sidebar'}
    onclick={() => (view.sidebarCollapsed = !view.sidebarCollapsed)}
  >
    <Icon name={view.sidebarCollapsed ? 'sidebar-collapse' : 'sidebar-expand'} />
  </Button>

  <h1 class="repo" title={session.repo?.root}>
    <Icon name="repo" />
    <span>{session.repo?.name}</span>
  </h1>

  <!-- The ref pickers and the mode toggle come in M4 (PRD §6.3). -->
  <div class="comparison" data-testid="comparison-bar">
    <span class="ref"><span class="muted">base:</span> {refLabel(session.comparison.base)}</span>
    <Icon name="arrow-left" />
    <span class="ref">
      <span class="muted">compare:</span>
      {compareLabel(session.comparison.compare)}
    </span>
  </div>

  <!-- The diff style and hide whitespace toggles come in M5 (PRD §6.6, §6.7). -->
  <div class="end"></div>
</header>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--base-size-12);
    min-height: var(--base-size-48);
    padding: var(--base-size-8) var(--base-size-12);
    border-bottom: var(--borderWidth-thin) solid var(--borderColor-default);
    background: var(--bgColor-muted);
  }

  .repo {
    display: flex;
    align-items: center;
    gap: var(--base-size-8);
    min-width: 0;
    margin: 0;
    font-size: var(--text-body-size-large);
    font-weight: var(--base-text-weight-semibold);
  }

  .repo > :global(.icon) {
    color: var(--fgColor-muted);
  }

  .repo span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .comparison {
    display: flex;
    align-items: center;
    gap: var(--base-size-8);
    min-width: 0;
    color: var(--fgColor-muted);
  }

  .ref {
    padding: var(--base-size-2) var(--base-size-8);
    border: var(--borderWidth-thin) solid var(--borderColor-default);
    border-radius: var(--borderRadius-medium);
    background: var(--bgColor-default);
    color: var(--fgColor-default);
    font-size: var(--text-body-size-small);
    white-space: nowrap;
  }

  .muted {
    color: var(--fgColor-muted);
  }

  .end {
    margin-left: auto;
  }
</style>
