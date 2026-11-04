<script lang="ts">
  import { getStores } from '../stores/context';
  import { SIDEBAR_WIDTHS } from '../stores/settings.svelte';
  import ErrorNotice from './ErrorNotice.svelte';
  import MainPane from './MainPane.svelte';
  import Sidebar from './Sidebar.svelte';
  import Toolbar from './Toolbar.svelte';

  /** The narrowest the main pane gets while dragging the divider. */
  const MAIN_MIN_WIDTH = 320;

  const { session, view, settings } = getStores();

  /** The width while dragging. It's saved when the drag ends. */
  let dragWidth = $state<number | null>(null);
  const width = $derived(dragWidth ?? settings.value.sidebarWidth);

  function clamp(value: number): number {
    const max = Math.min(SIDEBAR_WIDTHS.max, window.innerWidth - MAIN_MIN_WIDTH);
    return Math.round(Math.max(SIDEBAR_WIDTHS.min, Math.min(max, value)));
  }

  function startDrag(e: PointerEvent) {
    if (e.button !== 0) return;
    const divider = e.currentTarget as HTMLElement;
    const startX = e.clientX;
    const startWidth = width;
    divider.setPointerCapture(e.pointerId);
    dragWidth = startWidth;

    const move = (e: PointerEvent) => (dragWidth = clamp(startWidth + e.clientX - startX));
    const end = () => {
      divider.removeEventListener('pointermove', move);
      divider.removeEventListener('pointerup', end);
      divider.removeEventListener('pointercancel', end);
      const sidebarWidth = dragWidth;
      if (sidebarWidth !== null && sidebarWidth !== settings.value.sidebarWidth) {
        void settings.update({ sidebarWidth }).finally(() => (dragWidth = null));
      } else {
        dragWidth = null;
      }
    };
    divider.addEventListener('pointermove', move);
    divider.addEventListener('pointerup', end);
    divider.addEventListener('pointercancel', end);
  }
</script>

<div class="repo-view" class:dragging={dragWidth !== null}>
  <Toolbar />
  {#if session.notice}
    <div class="notice">
      <ErrorNotice error={session.notice} ondismiss={() => (session.notice = null)} />
    </div>
  {/if}
  <div class="body">
    {#if !view.sidebarCollapsed}
      <aside id="sidebar" class="sidebar" style:width="{width}px" aria-label="Changed files">
        <Sidebar />
      </aside>
      <div
        class="divider"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize sidebar"
        aria-valuenow={width}
        aria-valuemin={SIDEBAR_WIDTHS.min}
        aria-valuemax={SIDEBAR_WIDTHS.max}
        onpointerdown={startDrag}
      ></div>
    {/if}
    <MainPane />
  </div>
</div>

<style>
  .repo-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .dragging {
    cursor: col-resize;
    user-select: none;
  }

  .notice {
    padding: var(--base-size-8) var(--base-size-12) 0;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .body > :global(.main) {
    flex: 1;
  }

  .sidebar {
    flex: none;
    overflow: auto;
    background: var(--bgColor-default);
  }

  .divider {
    flex: none;
    position: relative;
    width: var(--borderWidth-thin);
    background: var(--borderColor-default);
    cursor: col-resize;
    touch-action: none;
  }

  /* A wider grab area than the 1px line. */
  .divider::before {
    content: '';
    position: absolute;
    inset: 0 calc(-1 * var(--base-size-4));
    z-index: 1;
  }

  .divider:hover,
  .dragging .divider {
    background: var(--borderColor-accent-emphasis);
  }
</style>
