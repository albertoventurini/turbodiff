<script lang="ts">
  import { getStores } from '../stores/context';
  import type { CompareSide, Mode } from '../types';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import ModeToggle from './ModeToggle.svelte';
  import RefPicker from './RefPicker.svelte';

  const { session, view } = getStores();

  const comparison = $derived(session.comparison);
  const canSwap = $derived(comparison.compare.kind === 'commit');

  function setBase(side: CompareSide) {
    if (side.kind === 'commit') void session.compare({ ...comparison, base: side.spec });
  }

  function setCompare(compare: CompareSide) {
    void session.compare({ ...comparison, compare });
  }

  function setMode(mode: Mode) {
    void session.compare({ ...comparison, mode });
  }

  function swap() {
    if (comparison.compare.kind !== 'commit') return;
    void session.compare({
      ...comparison,
      base: comparison.compare.spec,
      compare: { kind: 'commit', spec: comparison.base },
    });
  }
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

  <div class="comparison" data-testid="comparison-bar">
    <RefPicker side="base" value={{ kind: 'commit', spec: comparison.base }} onselect={setBase} />
    <span class="arrow"><Icon name="arrow-left" /></span>
    <RefPicker side="compare" value={comparison.compare} onselect={setCompare} />
    <Button
      variant="invisible"
      size="small"
      aria-label="Swap base and compare"
      title={canSwap
        ? 'Swap base and compare'
        : "The working tree can't be the base, so it can't be swapped"}
      disabled={!canSwap}
      onclick={swap}
    >
      <Icon name="arrow-switch" />
    </Button>
    <ModeToggle value={comparison.mode} onchange={setMode} />
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

  .arrow {
    display: flex;
    flex: none;
  }

  .end {
    margin-left: auto;
  }
</style>
