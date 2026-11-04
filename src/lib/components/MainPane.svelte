<script lang="ts">
  import { errorDetail } from '../errors';
  import { compareLabel, refLabel } from '../refs';
  import { getStores } from '../stores/context';
  import Button from './Button.svelte';
  import EmptyState from './EmptyState.svelte';
  import Notice from './Notice.svelte';

  const { session, view } = getStores();

  const base = $derived(refLabel(session.comparison.base));
  const compare = $derived(compareLabel(session.comparison.compare));
  const isDefault = $derived(
    session.comparison.base === 'HEAD' && session.comparison.compare.kind === 'workTree',
  );
</script>

<main class="main">
  {#if session.error}
    {@const error = session.error}
    {#if error.kind === 'refNotFound'}
      <EmptyState icon="alert" title="Invalid ref" tone="danger">
        <p><code>{error.name}</code> isn't a branch, tag or commit in this repository.</p>
        {#snippet actions()}
          <Button onclick={() => void session.resetComparison()}>
            Compare working tree with HEAD
          </Button>
        {/snippet}
      </EmptyState>
    {:else}
      <EmptyState icon="stop" title="Couldn't list the changes" tone="danger">
        <p>{errorDetail(error) ?? error.kind}</p>
        {#snippet actions()}
          <Button onclick={() => void session.compare(session.comparison)}>Try again</Button>
        {/snippet}
      </EmptyState>
    {/if}
  {:else if session.changes}
    {#if session.changes.noMergeBase}
      <div class="notice">
        <Notice>
          <strong>No merge base.</strong>
          <code>{base}</code> and <code>{compare}</code> have no common history, so this is a direct (two-dot)
          comparison.
        </Notice>
      </div>
    {/if}
    {#if session.changes.files.length === 0}
      <EmptyState icon="check-circle" title="No changes">
        {#if isDefault}
          <p>Working tree matches HEAD.</p>
        {:else if session.comparison.mode === 'threeDot' && !session.changes.noMergeBase}
          <p>
            Nothing has changed on <code>{compare}</code> since it split off from
            <code>{base}</code>.
          </p>
        {:else}
          <p><code>{compare}</code> matches <code>{base}</code>.</p>
        {/if}
      </EmptyState>
    {:else}
      <!--
        The diff view comes in M5 (PRD §6.6). It shows only `view.selected` when
        a file is selected, and otherwise all files, keeping `view.current` set
        to the file at the top of the pane.
      -->
      <div class="placeholder" data-testid="diff-placeholder">
        {#if view.selected}
          {view.selected}
        {:else}
          {session.changes.totals.files} changed files
        {/if}
      </div>
    {/if}
  {:else if session.loading}
    <p class="loading" role="status">Loading changes…</p>
  {/if}
</main>

<style>
  .main {
    min-width: 0;
    min-height: 0;
    overflow: auto;
  }

  .notice {
    margin: var(--base-size-16) var(--base-size-16) 0;
  }

  .placeholder {
    margin: var(--base-size-16);
    padding: var(--base-size-40);
    border: var(--borderWidth-thin) dashed var(--borderColor-default);
    border-radius: var(--borderRadius-medium);
    color: var(--fgColor-muted);
    text-align: center;
  }

  /* Shown only if loading takes a moment, so a quick load doesn't flicker. */
  .loading {
    margin: var(--base-size-40) 0;
    color: var(--fgColor-muted);
    text-align: center;
    opacity: 0;
    animation: appear 0s 400ms forwards;
  }

  @keyframes appear {
    to {
      opacity: 1;
    }
  }
</style>
