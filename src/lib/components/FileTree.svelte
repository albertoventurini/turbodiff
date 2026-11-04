<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import { buildTree, comparePaths, treeRows, type TreeRow } from '../fileTree';
  import { getStores } from '../stores/context';
  import type { FileEntry } from '../types';
  import Icon from './Icon.svelte';
  import StatusBadge, { STATUS_NAMES } from './StatusBadge.svelte';

  let { files }: { files: FileEntry[] } = $props();

  const { view } = getStores();

  /** Folders the user collapsed, by path. Kept while the files change. */
  const collapsed = new SvelteSet<string>();

  const tree = $derived(buildTree(files));
  const rows = $derived<TreeRow[]>(
    view.fileTreeMode === 'tree'
      ? treeRows(tree, collapsed)
      : [...files]
          .sort((a, b) => comparePaths(a.path, b.path))
          .map((file) => ({ node: { kind: 'file', name: file.path, file }, depth: 0 })),
  );

  function toggleFolder(path: string) {
    if (!collapsed.delete(path)) collapsed.add(path);
  }

  function describe(file: FileEntry): string {
    const status = STATUS_NAMES[file.status];
    return file.oldPath ? `${status}: ${file.oldPath} → ${file.path}` : `${status}: ${file.path}`;
  }

  /** `src/app/` and `main.rs` of `src/app/main.rs`. */
  function split(path: string): [string, string] {
    const slash = path.lastIndexOf('/') + 1;
    return [path.slice(0, slash), path.slice(slash)];
  }
</script>

<ul class="file-tree" aria-label="Changed files">
  {#each rows as { node, depth } (node.kind === 'file' ? `f:${node.file.path}` : `d:${node.path}`)}
    <li style="--depth: {depth}">
      {#if node.kind === 'folder'}
        {@const open = !collapsed.has(node.path)}
        <button
          type="button"
          class="row folder"
          aria-expanded={open}
          title={node.path}
          onclick={() => toggleFolder(node.path)}
        >
          <span class="chevron"><Icon name={open ? 'chevron-down' : 'chevron-right'} /></span>
          <span class="folder-icon"><Icon name="file-directory-fill" /></span>
          <span class="name">{node.name}</span>
        </button>
      {:else}
        {@const file = node.file}
        {@const [dir, base] = view.fileTreeMode === 'flat' ? split(file.path) : ['', node.name]}
        <button
          type="button"
          class="row file"
          class:selected={view.selected === file.path}
          aria-pressed={view.selected === file.path}
          aria-current={view.selected === null && view.current === file.path ? 'true' : undefined}
          title={describe(file)}
          onclick={() => view.toggleSelected(file.path)}
        >
          <StatusBadge status={file.status} />
          <span class="path">
            {#if dir}<span class="dir">{dir}</span>{/if}<span class="name">{base}</span>
          </span>
          <span class="counts">
            {#if file.binary}
              <span class="bin">bin</span>
            {:else}
              {#if file.additions > 0}<span class="additions">+{file.additions}</span>{/if}
              {#if file.deletions > 0}<span class="deletions">−{file.deletions}</span>{/if}
            {/if}
          </span>
        </button>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .file-tree {
    margin: 0;
    padding: 0 var(--base-size-8) var(--base-size-8);
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--base-size-6);
    width: 100%;
    min-height: var(--control-small-size);
    padding: 0 var(--base-size-8) 0 calc(var(--base-size-8) + var(--depth) * var(--base-size-16));
    border: 0;
    border-radius: var(--borderRadius-medium);
    background: none;
    font-size: var(--text-body-size-small);
    text-align: left;
    cursor: pointer;
  }

  .row:hover {
    background: var(--control-transparent-bgColor-hover);
  }

  .chevron,
  .folder-icon {
    display: flex;
    color: var(--fgColor-muted);
  }

  .folder-icon {
    color: var(--treeViewItem-leadingVisual-iconColor-rest);
  }

  /* Files line up with the folder names above them, past the chevron. */
  .file {
    padding-left: calc(
      var(--base-size-8) + var(--depth) * var(--base-size-16) + var(--base-size-4)
    );
  }

  .file.selected {
    background: var(--control-transparent-bgColor-selected);
    font-weight: var(--base-text-weight-semibold);
  }

  /* The file at the top of the all-files view: a bar at the left edge. */
  .file[aria-current='true'] {
    box-shadow: inset var(--borderWidth-thick) 0 0 var(--borderColor-accent-emphasis);
  }

  .path {
    display: flex;
    flex: 1;
    min-width: 0;
    white-space: nowrap;
  }

  .dir,
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dir {
    flex: 0 1 auto;
    color: var(--fgColor-muted);
  }

  /* The file name gives way only once the folders are hidden. */
  .file .name {
    flex: 0 0 auto;
    max-width: 100%;
  }

  .counts {
    display: flex;
    flex: none;
    gap: var(--base-size-4);
    font-family: var(--fontStack-monospace);
    font-size: var(--text-caption-size);
    font-weight: var(--base-text-weight-normal);
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
