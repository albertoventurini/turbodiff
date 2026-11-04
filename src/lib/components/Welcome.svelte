<script lang="ts">
  import * as api from '../api';
  import { ApiError } from '../api';
  import { getStores } from '../stores/context';
  import Button from './Button.svelte';
  import ErrorNotice from './ErrorNotice.svelte';
  import Icon from './Icon.svelte';

  const { session, recent } = getStores();

  async function open(path: string) {
    try {
      await api.openRepo(path);
    } catch (err) {
      if (err instanceof ApiError) session.notice = err.error;
      else throw err;
    }
  }
</script>

<main class="welcome">
  <div class="content">
    {#if session.notice}
      <div class="notice">
        <ErrorNotice error={session.notice} ondismiss={() => (session.notice = null)} />
      </div>
    {/if}

    <header>
      <span class="logo"><Icon name="git-compare" size={32} /></span>
      <h1>TurboDiff</h1>
    </header>

    <div class="open">
      <Button variant="primary" onclick={() => void api.showOpenDialog()}>
        <Icon name="file-directory" />
        Open Repository…
      </Button>
      <p class="hint">or drop a folder onto this window</p>
    </div>

    {#if recent.items.length > 0}
      <section class="recent" aria-labelledby="recent-heading">
        <h2 id="recent-heading">Recent repositories</h2>
        <ul>
          {#each recent.items as item (item.path)}
            <li class:unavailable={!item.available}>
              {#if item.available}
                <button type="button" class="entry" onclick={() => void open(item.path)}>
                  <Icon name="repo" />
                  <span class="name">{item.name}</span>
                  <span class="path">{item.path}</span>
                </button>
              {:else}
                <div class="entry">
                  <Icon name="repo" />
                  <span class="name">{item.name}</span>
                  <span class="path">{item.path}</span>
                  <span class="label">Unavailable</span>
                </div>
                <button
                  type="button"
                  class="remove"
                  aria-label="Remove {item.name} from recent repositories"
                  title="Remove from list"
                  onclick={() => void recent.remove(item.path)}
                >
                  <Icon name="x" />
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>
</main>

<style>
  .welcome {
    display: flex;
    justify-content: center;
    height: 100%;
    overflow-y: auto;
    padding: var(--base-size-64) var(--base-size-24) var(--base-size-40);
  }

  .content {
    width: 100%;
    max-width: 32rem;
  }

  .notice {
    margin-bottom: var(--base-size-24);
  }

  header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--base-size-8);
  }

  .logo {
    display: flex;
    color: var(--fgColor-muted);
  }

  h1 {
    margin: 0;
    font-size: var(--text-title-size-large);
    font-weight: var(--text-title-weight-large);
    line-height: var(--text-title-lineHeight-large);
  }

  .open {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--base-size-8);
    margin-top: var(--base-size-24);
  }

  .hint {
    margin: 0;
    color: var(--fgColor-muted);
    font-size: var(--text-body-size-small);
  }

  .recent {
    margin-top: var(--base-size-40);
  }

  h2 {
    margin: 0 0 var(--base-size-8);
    font-size: var(--text-body-size-medium);
    font-weight: var(--base-text-weight-semibold);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border: var(--borderWidth-thin) solid var(--borderColor-default);
    border-radius: var(--borderRadius-medium);
    overflow: hidden;
  }

  li {
    display: flex;
    align-items: center;
  }

  li + li {
    border-top: var(--borderWidth-thin) solid var(--borderColor-muted);
  }

  .entry {
    display: grid;
    grid-template-columns: auto 1fr auto;
    column-gap: var(--base-size-8);
    align-items: center;
    flex: 1;
    min-width: 0;
    padding: var(--base-size-8) var(--base-size-16);
    border: 0;
    background: none;
    text-align: left;
  }

  button.entry {
    cursor: pointer;
  }

  button.entry:hover {
    background: var(--bgColor-muted);
  }

  .entry > :global(.icon) {
    color: var(--fgColor-muted);
  }

  .name {
    font-weight: var(--base-text-weight-semibold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .path {
    grid-column: 2 / -1;
    color: var(--fgColor-muted);
    font-size: var(--text-body-size-small);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .unavailable .name,
  .unavailable .path {
    color: var(--fgColor-disabled);
  }

  .label {
    grid-row: 1;
    grid-column: 3;
    padding: 0 var(--base-size-8);
    border: var(--borderWidth-thin) solid var(--borderColor-default);
    border-radius: var(--borderRadius-full);
    color: var(--fgColor-muted);
    font-size: var(--text-body-size-small);
  }

  .remove {
    display: flex;
    margin-right: var(--base-size-8);
    padding: var(--base-size-4);
    border: 0;
    border-radius: var(--borderRadius-medium);
    background: none;
    color: var(--fgColor-muted);
    cursor: pointer;
  }

  .remove:hover {
    background: var(--button-invisible-bgColor-hover);
    color: var(--fgColor-default);
  }
</style>
