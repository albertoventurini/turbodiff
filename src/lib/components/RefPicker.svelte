<script lang="ts">
  import { tick } from 'svelte';
  import * as api from '../api';
  import { ApiError } from '../api';
  import { errorTitle } from '../errors';
  import {
    compareLabel,
    isSelectedCommit,
    isSelectedRef,
    looksLikeSha,
    selectionTab,
    type RefTab,
  } from '../refs';
  import { getStores } from '../stores/context';
  import { relativeTime } from '../time';
  import type { CommitItem, CompareSide, RefItem, Versioned } from '../types';
  import Icon from './Icon.svelte';

  /** Commits per page of the Commits tab, and the most a message search returns. */
  const PAGE = 50;
  /** How close to the end of the list, in pixels, the next page starts loading. */
  const LOAD_MARGIN = 200;
  /** How long typing pauses before the backend is searched, in milliseconds. */
  const SEARCH_DELAY = 150;

  const TABS: { id: RefTab; label: string }[] = [
    { id: 'branches', label: 'Branches' },
    { id: 'tags', label: 'Tags' },
    { id: 'commits', label: 'Commits' },
  ];

  let {
    side,
    value,
    onselect,
  }: {
    /** Only the compare picker offers the working tree (PRD §6.3). */
    side: 'base' | 'compare';
    value: CompareSide;
    onselect: (value: CompareSide) => void;
  } = $props();

  const { session } = getStores();
  const id = $derived(`ref-picker-${side}`);
  const head = $derived(session.repo?.head ?? { branch: null, commit: null });
  const spec = $derived(value.kind === 'commit' ? value.spec : null);

  let open = $state(false);
  let query = $state('');
  let tab = $state<RefTab>('branches');
  /** Whether the user picked a tab since the picker opened. */
  let tabChosen = false;

  // Kept between openings, so the tab of the selection is known at once.
  let branches = $state<RefItem[] | null>(null);
  let tags = $state<RefItem[] | null>(null);
  let refsLoading = $state(false);
  let commits = $state<CommitItem[]>([]);
  let commitsDone = $state(false);
  let commitsLoading = $state(false);
  /** Message search results, for `searched`. */
  let found = $state<CommitItem[]>([]);
  let searched = $state('');
  let searching = $state(false);
  /** Commits whose SHA starts with the search text. */
  let shaMatches = $state<CommitItem[]>([]);
  let error = $state<string | null>(null);

  let root = $state<HTMLElement>();
  let trigger = $state<HTMLButtonElement>();
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLElement>();
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  /** Bumped on each opening and closing, so responses for an earlier one are dropped. */
  let opening = 0;
  /** Bumped on each search, so older results are dropped. */
  let searchRequest = 0;

  const needle = $derived(query.trim());
  const filteredRefs = $derived.by(() => {
    const items = (tab === 'branches' ? branches : tags) ?? [];
    const lower = needle.toLowerCase();
    return lower ? items.filter((item) => item.name.toLowerCase().includes(lower)) : items;
  });
  const shownCommits = $derived.by(() => {
    if (!needle) return commits;
    const seen = new Set(shaMatches.map((commit) => commit.sha));
    return [...shaMatches, ...found.filter((commit) => !seen.has(commit.sha))];
  });
  const loadingList = $derived(
    tab === 'commits'
      ? needle
        ? searching
        : commitsLoading
      : refsLoading && (tab === 'branches' ? branches : tags) === null,
  );

  /** The value of a current response; a stale or failed one is dropped. */
  async function load<T>(response: Promise<Versioned<T>>): Promise<T | null> {
    try {
      return session.current(await response);
    } catch (err) {
      if (err instanceof ApiError && err.error.kind !== 'cancelled') {
        error = errorTitle(err.error);
      } else if (!(err instanceof ApiError)) {
        error = String(err);
      }
      return null;
    }
  }

  async function show() {
    const current = ++opening;
    open = true;
    query = '';
    found = [];
    searched = '';
    shaMatches = [];
    error = null;
    tabChosen = false;
    commits = [];
    commitsDone = false;
    commitsLoading = false;
    searching = false;
    tab = spec === null ? 'branches' : selectionTab(spec, head, branches, tags);
    await tick();
    input?.focus();

    refsLoading = true;
    const [b, t] = await Promise.all([load(api.listBranches()), load(api.listTags())]);
    if (current !== opening) return;
    refsLoading = false;
    if (b) branches = b;
    if (t) tags = t;
    if (!tabChosen && spec !== null) setTab(selectionTab(spec, head, branches, tags), false);
    if (tab === 'commits') void loadMoreCommits();
  }

  function hide(refocus = false) {
    open = false;
    opening++;
    clearTimeout(searchTimer);
    if (refocus) trigger?.focus();
  }

  function setTab(next: RefTab, chosen = true) {
    tabChosen ||= chosen;
    if (next === tab) return;
    tab = next;
    if (list) list.scrollTop = 0;
    if (next === 'commits') {
      if (needle && searched !== needle) void search();
      else if (commits.length === 0) void loadMoreCommits();
    }
  }

  async function loadMoreCommits() {
    if (commitsLoading || commitsDone) return;
    const current = opening;
    commitsLoading = true;
    const page = await load(api.listCommits(commits.length, PAGE));
    if (current !== opening) return;
    commitsLoading = false;
    // A failed page isn't retried on every scroll.
    commits = [...commits, ...(page ?? [])];
    commitsDone = page === null || page.length < PAGE;
  }

  function oninput() {
    clearTimeout(searchTimer);
    if (!needle) {
      shaMatches = [];
      found = [];
      searched = '';
      return;
    }
    searchTimer = setTimeout(() => void search(), SEARCH_DELAY);
  }

  /** Looks the search text up as a SHA and, on the Commits tab, in commit messages. */
  async function search() {
    const text = needle;
    const current = [opening, ++searchRequest].join();
    const bySha = looksLikeSha(text) ? load(api.resolveCommitPrefix(text)) : Promise.resolve([]);
    const byMessage = tab === 'commits' ? load(api.searchCommits(text, PAGE)) : null;
    searching = byMessage !== null;
    const [sha, message] = await Promise.all([bySha, byMessage]);
    if (current !== [opening, searchRequest].join()) return;
    searching = false;
    shaMatches = sha ?? [];
    if (message) {
      found = message;
      searched = text;
    }
  }

  function onscroll() {
    if (!list || tab !== 'commits' || needle) return;
    if (list.scrollTop + list.clientHeight >= list.scrollHeight - LOAD_MARGIN) {
      void loadMoreCommits();
    }
  }

  function choose(next: CompareSide) {
    hide(true);
    if (next.kind !== value.kind || (next.kind === 'commit' && next.spec !== spec)) onselect(next);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      // Keeps Esc from also deselecting a file.
      e.preventDefault();
      hide(true);
    }
  }

  $effect(() => {
    if (!open) return;
    const outside = (e: PointerEvent) => {
      if (root && !root.contains(e.target as Node)) hide();
    };
    window.addEventListener('pointerdown', outside);
    return () => window.removeEventListener('pointerdown', outside);
  });
</script>

{#snippet check(selected: boolean)}
  <span class="check">
    {#if selected}<Icon name="check" />{/if}
  </span>
{/snippet}

{#snippet commitItem(commit: CommitItem)}
  {@const selected = spec !== null && isSelectedCommit(spec, commit, head)}
  <li>
    <button
      type="button"
      class="item commit"
      aria-current={selected || undefined}
      title={commit.sha}
      onclick={() => choose({ kind: 'commit', spec: commit.sha })}
    >
      {@render check(selected)}
      <span class="text">
        <span class="line">
          <code class="sha">{commit.shortSha}</code>
          <span class="subject">{commit.subject}</span>
        </span>
        <span class="meta">{commit.authorName} · {relativeTime(commit.time)}</span>
      </span>
    </button>
  </li>
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="ref-picker" bind:this={root} {onkeydown}>
  <button
    type="button"
    class="trigger"
    bind:this={trigger}
    aria-haspopup="dialog"
    aria-expanded={open}
    aria-controls={open ? id : undefined}
    title={spec ?? undefined}
    onclick={() => (open ? hide() : void show())}
  >
    <span class="side">{side}:</span>
    <span class="label">{compareLabel(value)}</span>
    <Icon name="triangle-down" />
  </button>

  {#if open}
    <div class="popup" {id} role="dialog" aria-label="Choose the {side}">
      <div class="title">
        <span>Choose the {side}</span>
        <button type="button" class="close" aria-label="Close" onclick={() => hide(true)}>
          <Icon name="x" />
        </button>
      </div>
      <label class="search">
        <Icon name="search" />
        <input
          type="text"
          bind:this={input}
          bind:value={query}
          {oninput}
          placeholder="Find a branch, tag or commit"
          aria-label="Find a branch, tag or commit"
          autocomplete="off"
          spellcheck="false"
        />
      </label>

      {#if side === 'compare'}
        <button
          type="button"
          class="item pinned"
          aria-current={value.kind === 'workTree' || undefined}
          onclick={() => choose({ kind: 'workTree' })}
        >
          {@render check(value.kind === 'workTree')}
          <Icon name="file-diff" />
          <span class="name">Working tree</span>
        </button>
      {/if}

      <div class="tabs" role="tablist">
        {#each TABS as t (t.id)}
          <button
            type="button"
            role="tab"
            id="{id}-{t.id}"
            aria-selected={tab === t.id}
            aria-controls="{id}-panel"
            onclick={() => setTab(t.id)}
          >
            <Icon
              name={t.id === 'branches' ? 'git-branch' : t.id === 'tags' ? 'tag' : 'git-commit'}
            />
            {t.label}
          </button>
        {/each}
      </div>

      <div
        class="list"
        id="{id}-panel"
        role="tabpanel"
        aria-labelledby="{id}-{tab}"
        bind:this={list}
        {onscroll}
      >
        {#if tab !== 'commits' && shaMatches.length > 0}
          <p class="heading">Commits with this SHA</p>
          <ul>
            {#each shaMatches as commit (commit.sha)}
              {@render commitItem(commit)}
            {/each}
          </ul>
          <p class="heading">{tab === 'branches' ? 'Branches' : 'Tags'}</p>
        {/if}

        {#if tab === 'commits'}
          <ul>
            {#each shownCommits as commit (commit.sha)}
              {@render commitItem(commit)}
            {/each}
          </ul>
        {:else}
          <ul>
            {#each filteredRefs as ref (ref.fullName)}
              {@const selected = spec !== null && isSelectedRef(spec, ref)}
              <li>
                <button
                  type="button"
                  class="item"
                  aria-current={selected || undefined}
                  title={ref.fullName}
                  onclick={() => choose({ kind: 'commit', spec: ref.fullName })}
                >
                  {@render check(selected)}
                  <span class="name">{ref.name}</span>
                  {#if ref.current}<span class="label-current">current</span>{/if}
                  <span class="date">{relativeTime(ref.time)}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}

        {#if error}
          <p class="hint error" role="alert">{error}</p>
        {:else if loadingList}
          <p class="hint" role="status">Loading…</p>
        {:else if (tab === 'commits' ? shownCommits : filteredRefs).length === 0}
          <p class="hint">
            {#if needle}
              Nothing matches “{needle}”.
            {:else}
              No {tab} yet.
            {/if}
          </p>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .ref-picker {
    position: relative;
    min-width: 0;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: var(--base-size-4);
    max-width: 100%;
    height: var(--control-small-size);
    padding: 0 var(--control-small-paddingInline-condensed);
    border: var(--borderWidth-thin) solid var(--button-default-borderColor-rest);
    border-radius: var(--borderRadius-medium);
    background: var(--button-default-bgColor-rest);
    box-shadow: var(--button-default-shadow-resting);
    font-size: var(--text-body-size-small);
    white-space: nowrap;
    cursor: pointer;
  }

  .trigger:hover {
    background: var(--button-default-bgColor-hover);
    border-color: var(--button-default-borderColor-hover);
  }

  .trigger[aria-expanded='true'] {
    background: var(--button-default-bgColor-active);
  }

  .trigger > :global(.icon) {
    color: var(--fgColor-muted);
  }

  .side {
    color: var(--fgColor-muted);
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: var(--base-text-weight-semibold);
  }

  .popup {
    position: absolute;
    top: calc(100% + var(--base-size-4));
    left: 0;
    z-index: 10;
    display: flex;
    flex-direction: column;
    width: var(--overlay-width-small);
    border: var(--borderWidth-thin) solid var(--overlay-borderColor);
    border-radius: var(--overlay-borderRadius);
    background: var(--overlay-bgColor);
    box-shadow: var(--shadow-floating-small);
    font-size: var(--text-body-size-small);
  }

  .title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--base-size-8) var(--base-size-8) var(--base-size-4) var(--base-size-12);
    font-weight: var(--base-text-weight-semibold);
  }

  .close {
    display: flex;
    padding: var(--base-size-4);
    border: 0;
    border-radius: var(--borderRadius-small);
    background: none;
    color: var(--fgColor-muted);
    cursor: pointer;
  }

  .close:hover {
    background: var(--control-transparent-bgColor-hover);
    color: var(--fgColor-default);
  }

  .search {
    display: flex;
    align-items: center;
    gap: var(--base-size-8);
    margin: var(--base-size-4) var(--base-size-8) var(--base-size-8);
    padding: 0 var(--base-size-8);
    height: var(--control-medium-size);
    border: var(--borderWidth-thin) solid var(--control-borderColor-rest);
    border-radius: var(--borderRadius-medium);
    background: var(--bgColor-default);
    color: var(--fgColor-muted);
  }

  .search:focus-within {
    outline: var(--focus-outline);
    outline-offset: calc(-1 * var(--borderWidth-thin));
  }

  .search input {
    flex: 1;
    min-width: 0;
    padding: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--fgColor-default);
    font: inherit;
  }

  .search input::placeholder {
    color: var(--fgColor-muted);
  }

  .tabs {
    display: flex;
    gap: var(--base-size-4);
    padding: 0 var(--base-size-8);
    border-bottom: var(--borderWidth-thin) solid var(--borderColor-muted);
  }

  .tabs button {
    display: flex;
    align-items: center;
    gap: var(--base-size-4);
    padding: var(--base-size-6) var(--base-size-8);
    border: 0;
    border-bottom: var(--borderWidth-thick) solid transparent;
    margin-bottom: calc(-1 * var(--borderWidth-thin));
    background: none;
    color: var(--fgColor-muted);
    cursor: pointer;
  }

  .tabs button:hover {
    color: var(--fgColor-default);
    border-bottom-color: var(--underlineNav-borderColor-hover);
  }

  .tabs button[aria-selected='true'] {
    color: var(--fgColor-default);
    font-weight: var(--base-text-weight-semibold);
    border-bottom-color: var(--underlineNav-borderColor-active);
  }

  .list {
    max-height: var(--overlay-height-medium);
    overflow-y: auto;
    padding: var(--base-size-4) 0;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--base-size-8);
    width: 100%;
    min-height: var(--control-medium-size);
    padding: var(--base-size-4) var(--base-size-12) var(--base-size-4) var(--base-size-8);
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }

  .item:hover {
    background: var(--control-transparent-bgColor-hover);
  }

  .item > :global(.icon) {
    color: var(--fgColor-muted);
  }

  .pinned {
    border-bottom: var(--borderWidth-thin) solid var(--borderColor-muted);
  }

  .check {
    display: flex;
    flex: none;
    width: var(--base-size-16);
  }

  .name,
  .subject {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label-current {
    flex: none;
    padding: 0 var(--base-size-6);
    border: var(--borderWidth-thin) solid var(--borderColor-default);
    border-radius: var(--borderRadius-full);
    color: var(--fgColor-muted);
    font-size: var(--text-caption-size);
  }

  .date {
    flex: none;
    margin-left: auto;
    padding-left: var(--base-size-8);
    color: var(--fgColor-muted);
    font-size: var(--text-caption-size);
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .line {
    display: flex;
    gap: var(--base-size-8);
    min-width: 0;
  }

  .sha {
    flex: none;
    color: var(--fgColor-muted);
    font-family: var(--fontStack-monospace);
    font-size: var(--text-caption-size);
    line-height: inherit;
  }

  .meta {
    overflow: hidden;
    color: var(--fgColor-muted);
    font-size: var(--text-caption-size);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .heading {
    margin: 0;
    padding: var(--base-size-4) var(--base-size-12);
    color: var(--fgColor-muted);
    font-size: var(--text-caption-size);
    font-weight: var(--base-text-weight-semibold);
  }

  .hint {
    margin: 0;
    padding: var(--base-size-12);
    color: var(--fgColor-muted);
    text-align: center;
  }

  .error {
    color: var(--fgColor-danger);
  }
</style>
