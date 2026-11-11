<script lang="ts" module>
  /** Fixed heights, in pixels, of the rows that never wrap. */
  export const HEADER_HEIGHT = 44;
  export const MESSAGE_HEIGHT = 64;
  /** The space between two files' cards, and at the top of the list. */
  export const GAP = 16;
  /** The bottom of a card. Matches `--borderRadius-medium`. */
  export const CARD_END = 6;
  /** Rows rendered above and below the viewport, in pixels. */
  const OVERSCAN = 400;
  /** How far beyond the viewport diffs are loaded, in viewport heights (TECH.md §7.3). */
  const LOAD_AHEAD = 2;
</script>

<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { claudeReference, type SelectedFile, type SelectedLine } from '../claudeRef';
  import { buildRows, type ExpandDirection, type Row } from '../diffRows';
  import { orderedFiles } from '../fileTree';
  import { findMatches, marksByCell, sameMatch, type FindMatch } from '../find';
  import { getStores } from '../stores/context';
  import { DiffStore } from '../stores/diffs.svelte';
  import { HighlightStore } from '../stores/highlights.svelte';
  import { resolveMode, THEMES } from '../theme/theme.svelte';
  import Button from './Button.svelte';
  import DiffRow from './DiffRow.svelte';
  import FileHeader from './FileHeader.svelte';
  import FindBar from './FindBar.svelte';
  import Icon from './Icon.svelte';

  const { session, view, settings, system } = getStores();
  const diffs = new DiffStore(session);
  const highlights = new HighlightStore(session);
  $effect(() => () => highlights.stop());

  /** The Shiki theme for the current theme and mode. */
  const shikiTheme = $derived.by(() => {
    const { theme, appearance } = settings.value;
    return THEMES[theme].shiki[resolveMode(appearance, system.dark)];
  });

  const files = $derived.by(() => {
    const all = session.changes?.files ?? [];
    if (view.selected !== null) return all.filter((file) => file.path === view.selected);
    return orderedFiles(all, view.fileTreeMode);
  });

  // A new file list replaces every diff; a refresh only the changed ones
  // (PRD §6.9).
  $effect.pre(() => {
    const { contextLines, diffAlgorithm } = settings.value;
    diffs.sync(session.listing, contextLines, diffAlgorithm);
    highlights.sync(session.listing);
  });
  $effect.pre(() => {
    const refresh = session.refresh;
    if (refresh) untrack(() => diffs.refresh(refresh, session.changes?.files ?? []));
  });

  const wrap = $derived(settings.value.lineWrapping);
  const lineHeight = $derived(Math.round((settings.value.fontSize * 20) / 13));
  const hunkHeight = $derived(lineHeight + 12);

  const rows = $derived(
    buildRows(files, {
      style: view.diffStyle,
      diffs: (path) => diffs.get(path),
      collapsed: view.collapsed,
      context: settings.value.contextLines,
    }),
  );
  const indexByKey = $derived(new Map(rows.map((row, i) => [row.key, i])));
  /** The header row of each file. */
  const headers = $derived(rows.flatMap((row, i) => (row.kind === 'header' ? [i] : [])));

  /** Row heights measured while wrapping, by key. */
  const measured = new SvelteMap<string, number>();

  function measures(row: Row): boolean {
    return wrap && (row.kind === 'line' || row.kind === 'split');
  }

  function fixedHeight(row: Row): number {
    switch (row.kind) {
      case 'header':
        return HEADER_HEIGHT;
      case 'hunk':
        return hunkHeight;
      case 'message':
        return MESSAGE_HEIGHT;
      case 'placeholder':
        return row.lines * lineHeight;
      case 'end':
        return GAP + (row.closed ? CARD_END : 0);
      default:
        return lineHeight;
    }
  }

  /** `offsets[i]` is the top of row `i`; the last entry is the total height. */
  const offsets = $derived.by(() => {
    const offsets = new Float64Array(rows.length + 1);
    rows.forEach((row, i) => {
      const height = measures(row) ? (measured.get(row.key) ?? lineHeight) : fixedHeight(row);
      offsets[i + 1] = offsets[i] + height;
    });
    return offsets;
  });

  /** The longest line, in characters, and the widest line number, in digits. */
  const extent = $derived.by(() => {
    const tab = settings.value.tabWidth;
    let chars = 0;
    let digits = 3;
    const visit = (text: string, no: number | null) => {
      let length = text.length;
      for (let i = text.indexOf('\t'); i !== -1; i = text.indexOf('\t', i + 1)) length += tab - 1;
      chars = Math.max(chars, length);
      if (no !== null) digits = Math.max(digits, String(no).length);
    };
    for (const row of rows) {
      if (row.kind === 'line') {
        visit(row.line.text, Math.max(row.line.oldNo ?? 0, row.line.newNo ?? 0));
      } else if (row.kind === 'split') {
        if (row.left) visit(row.left.text, row.left.no);
        if (row.right) visit(row.right.text, row.right.no);
      }
    }
    return { chars, digits };
  });

  let scroller = $state<HTMLElement>();
  let hscroll = $state<HTMLElement>();
  let scrollTop = $state(0);
  let viewport = $state(0);
  let width = $state(0);
  /** The viewport height, or the window's until it's measured. */
  const height = $derived(viewport || window.innerHeight);

  /** The position in the row list at the top of the viewport. */
  const top = $derived(Math.max(0, scrollTop - GAP));

  /** The row at `y`: the last one starting at or above it. */
  function rowAt(y: number): number {
    let lo = 0;
    let hi = rows.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (offsets[mid] <= y) lo = mid;
      else hi = mid - 1;
    }
    return lo;
  }

  const first = $derived(rowAt(top));
  const visible = $derived.by(() => {
    if (rows.length === 0) return [];
    const from = rowAt(Math.max(0, top - OVERSCAN));
    const to = rowAt(top + height + OVERSCAN);
    return rows.slice(from, to + 1).map((row, i) => ({ row, top: offsets[from + i] }));
  });

  /** The header of the file at the top, pinned while its own header is scrolled away. */
  const sticky = $derived.by(() => {
    if (rows.length === 0) return null;
    const { fileIndex } = rows[first];
    const header = headers[fileIndex];
    if (offsets[header] >= top) return null;
    const next = headers[fileIndex + 1];
    // The next file's header pushes this one out.
    const shift = next === undefined ? 0 : Math.min(0, offsets[next] - top - HEADER_HEIGHT);
    return { row: rows[header] as Extract<Row, { kind: 'header' }>, shift };
  });

  // The sidebar marks the file at the top of the all-files view.
  $effect(() => {
    if (view.selected !== null) return;
    const path = rows.length > 0 ? rows[first].file.path : null;
    if (untrack(() => view.current) !== path) view.current = path;
  });

  // Load the diffs of files near the viewport.
  $effect(() => {
    const from = rowAt(Math.max(0, top - LOAD_AHEAD * height));
    const to = rowAt(top + (LOAD_AHEAD + 1) * height);
    for (let i = from; i <= to && i < rows.length; i++) {
      const row = rows[i];
      if (row.kind === 'placeholder' || diffs.isStale(row.file.path)) {
        untrack(() => void diffs.load(row.file.path));
      }
    }
  });

  // Highlight the loaded diffs near the viewport. Rows show plain text until
  // the highlighting arrives.
  $effect(() => {
    if (rows.length === 0) return;
    const from = rows[rowAt(Math.max(0, top - LOAD_AHEAD * height))].fileIndex;
    const to = rows[rowAt(top + (LOAD_AHEAD + 1) * height)].fileIndex;
    const theme = shikiTheme;
    for (let i = from; i <= to; i++) {
      const loaded = diffs.get(files[i].path);
      // A stale diff no longer matches the file it would be highlighted from.
      if (loaded && loaded !== 'failed' && !loaded.stale) {
        untrack(() => highlights.request(files[i], loaded.diff, theme));
      }
    }
  });

  /**
   * What stays in place when rows above it change: the row at the top and the
   * scroll position relative to it, and the same for its file's header, in
   * case the row goes away (a placeholder replaced by the diff).
   */
  type Anchor = { key: string; delta: number; header: string; headerDelta: number };
  let anchor: Anchor | null = null;

  function takeAnchor(): Anchor | null {
    if (rows.length === 0) return null;
    const header = headers[rows[first].fileIndex];
    return {
      key: rows[first].key,
      delta: scrollTop - offsets[first],
      header: rows[header].key,
      headerDelta: scrollTop - offsets[header],
    };
  }

  /** The scroll position that puts `anchor` back in place. */
  function anchorScroll(anchor: Anchor): number {
    const row = indexByKey.get(anchor.key);
    if (row !== undefined) return offsets[row] + anchor.delta;
    const header = indexByKey.get(anchor.header);
    return header === undefined ? 0 : offsets[header] + anchor.headerDelta;
  }

  function onscroll() {
    if (!scroller) return;
    scrollTop = scroller.scrollTop;
    anchor = takeAnchor();
    placeCopyButton();
  }

  // Keep the anchor in place when rows change height, load or go away.
  $effect(() => {
    void offsets;
    untrack(() => {
      if (!scroller || !anchor) return;
      const target = anchorScroll(anchor);
      if (Math.abs(scroller.scrollTop - target) >= 1) {
        scroller.scrollTop = target;
        scrollTop = scroller.scrollTop;
      }
    });
  });

  // Single-file focus starts at the top; going back to all files restores the
  // place the user had there (PRD §6.6).
  let shownSelected = view.selected;
  let allFilesAnchor: Anchor | null = null;
  $effect.pre(() => {
    const selected = view.selected;
    untrack(() => {
      if (selected === shownSelected) return;
      if (shownSelected === null) allFilesAnchor = anchor;
      anchor =
        selected === null ? allFilesAnchor : { key: '', delta: 0, header: '', headerDelta: 0 };
      shownSelected = selected;
    });
  });

  function toggle(path: string) {
    const header = indexByKey.get(`${path}\0f`);
    // Collapsing the file whose header is pinned brings its header to the top.
    if (header !== undefined && offsets[header] < top) {
      const key = rows[header].key;
      anchor = { key, delta: GAP, header: key, headerDelta: GAP };
    }
    if (!view.collapsed.delete(path)) view.collapsed.add(path);
  }

  function expand(row: Extract<Row, { kind: 'hunk' }>, direction: ExpandDirection) {
    if (row.gap) void diffs.expand(row.file.path, row.gap, direction);
  }

  // Heights of wrapped rows, measured as they render.
  const observer =
    typeof ResizeObserver === 'undefined'
      ? null
      : new ResizeObserver((entries) => {
          for (const entry of entries) {
            const key = (entry.target as HTMLElement).dataset.key!;
            const height = (entry.target as HTMLElement).offsetHeight;
            if (height > 0 && measured.get(key) !== height) measured.set(key, height);
          }
        });

  function observe(row: Row) {
    return (node: HTMLElement) => {
      if (!observer || !measures(row)) return;
      observer.observe(node);
      return () => observer.unobserve(node);
    };
  }

  // Horizontal scrolling, while lines don't wrap: one offset for every line,
  // so headers and line numbers stay in place.
  let scrollX = $state(0);
  /** The width of 100 characters of code. */
  let probeWidth = $state(0);
  const charWidth = $derived(probeWidth / 100);
  /** How far rendered lines overflow their cells. */
  let overflow = $state(0);

  $effect(() => {
    void visible;
    void width;
    void view.diffStyle;
    if (!scroller || wrap) return;
    let most = 0;
    for (const content of scroller.querySelectorAll<HTMLElement>('.row .content')) {
      const room = content.parentElement!.clientWidth;
      most = Math.max(most, content.scrollWidth - room, extent.chars * charWidth - room);
    }
    overflow = Math.max(0, Math.ceil(most));
  });

  const maxScrollX = $derived(wrap ? 0 : overflow);
  $effect(() => {
    if (scrollX > maxScrollX) scrollX = maxScrollX;
  });

  function onhscroll() {
    if (hscroll) scrollX = hscroll.scrollLeft;
  }

  function wheel(node: HTMLElement) {
    const onwheel = (e: WheelEvent) => {
      const dx = e.deltaX || (e.shiftKey ? e.deltaY : 0);
      if (!dx || wrap || !hscroll || Math.abs(e.deltaY) > Math.abs(dx)) return;
      e.preventDefault();
      hscroll.scrollLeft += dx;
    };
    node.addEventListener('wheel', onwheel, { passive: false });
    return () => node.removeEventListener('wheel', onwheel);
  }

  // Find in page (PRD §6.10): the matches are found in the rows, since most of
  // them aren't rendered. While there's a query, every file's diff is loaded.
  let query = $state('');
  const searchAll = $derived(view.findOpen && query !== '');
  /** The files whose lines are searched: collapsed ones show none. */
  const searched = $derived(
    files.filter((file) => !file.submodule && !view.collapsed.has(file.path)),
  );
  $effect(() => {
    if (!searchAll) return;
    const paths = searched.map((file) => file.path);
    untrack(() => void diffs.loadAll(paths));
  });
  const searching = $derived(
    searchAll && searched.some((file) => diffs.get(file.path) === undefined),
  );

  const matches = $derived(view.findOpen ? findMatches(rows, query) : []);
  let currentMatch = $state<FindMatch | null>(null);
  const currentIndex = $derived.by(() => {
    const current = currentMatch;
    return current ? matches.findIndex((match) => sameMatch(match, current)) : -1;
  });
  const marks = $derived(marksByCell(matches, matches[currentIndex] ?? null));

  /**
   * Whether the current match is still the one picked for the query, rather
   * than one the user stepped to. Until they step, it follows the first
   * match at or below the top of the viewport as diffs load.
   */
  let picked = true;

  // A new query starts again from the top of the viewport.
  $effect.pre(() => {
    void query;
    currentMatch = null;
    picked = true;
  });

  $effect(() => {
    if (matches.length === 0 || (currentIndex !== -1 && !picked)) return;
    untrack(() => {
      const next = matches.find((match) => offsets[indexByKey.get(match.key)!] >= top);
      const match = next ?? matches[0];
      if (!currentMatch || !sameMatch(match, currentMatch)) void goTo(match);
    });
  });

  function step(by: 1 | -1) {
    if (matches.length === 0) return;
    picked = false;
    const from = currentIndex === -1 ? (by === 1 ? -1 : 0) : currentIndex;
    void goTo(matches[(from + by + matches.length) % matches.length]);
  }

  /** Makes `match` current and scrolls it into view, if it isn't in view. */
  async function goTo(match: FindMatch) {
    currentMatch = match;
    const row = indexByKey.get(match.key);
    if (!scroller || row === undefined) return;
    // The pinned header covers the top of the viewport.
    const above = offsets[row] + GAP < scroller.scrollTop + HEADER_HEIGHT;
    const below = offsets[row + 1] + GAP > scroller.scrollTop + height;
    if (above || below) {
      scroller.scrollTop = Math.max(0, offsets[row] + GAP - height / 3);
      onscroll();
    }
    await tick();
    const mark = scroller.querySelector<HTMLElement>('mark.current');
    const clip = mark?.closest<HTMLElement>('.clip');
    if (wrap || !hscroll || !mark || !clip) return;
    // Where the match starts and ends within the line, scrolled or not.
    const start = mark.getBoundingClientRect().left - clip.getBoundingClientRect().left + scrollX;
    const end = start + mark.offsetWidth;
    if (start < scrollX || end > scrollX + clip.clientWidth) {
      hscroll.scrollLeft = Math.max(0, start - clip.clientWidth / 3);
      scrollX = hscroll.scrollLeft;
    }
  }

  function closeFind() {
    view.findOpen = false;
    scroller?.focus();
  }

  // In split view, a selection stays on the side it started on.
  let selectSide = $state<'left' | 'right' | null>(null);
  function onpointerdown(e: PointerEvent) {
    const side = (e.target as Element).closest<HTMLElement>('[data-side]')?.dataset.side;
    selectSide = side === 'left' || side === 'right' ? side : null;
    pointerDown = true;
    copyAt = null;
  }

  // Copy for Claude (PRD §6.6): a button next to a selection of code, which
  // copies it with its file, line numbers and comparison.
  let list = $state<HTMLElement>();
  let pointerDown = false;
  /** Where the button goes, within the list; `null` hides it. */
  let copyAt = $state<{ top: number; right: number } | null>(null);
  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  /** The diff's selection, if it's in the rows. */
  function selectionRange(): Range | null {
    const selection = document.getSelection();
    if (!scroller || !selection || selection.isCollapsed || selection.rangeCount === 0) return null;
    const range = selection.getRangeAt(0);
    return scroller.contains(range.commonAncestorContainer) ? range : null;
  }

  /** The length of the text in `content` up to a boundary point inside it. */
  function offsetIn(content: Node, node: Node, offset: number): number {
    const before = document.createRange();
    before.selectNodeContents(content);
    before.setEnd(node, offset);
    return before.toString().length;
  }

  /** The line shown by one cell of a row; in split view, only on `side`. */
  function lineOf(row: Row, cellSide: string | undefined, side: string | null) {
    if (row.kind === 'line') return row.line;
    if (row.kind !== 'split' || (side !== null && cellSide !== side)) return null;
    const left = cellSide === 'left';
    const cell = left ? row.left : row.right;
    if (!cell) return null;
    return { ...cell, oldNo: left ? cell.no : null, newNo: left ? null : cell.no };
  }

  type SelectedPart = { row: Row; cell: HTMLElement; line: SelectedLine };

  /** The selected part of every rendered line the selection touches, in order. */
  function selectedParts(): SelectedPart[] {
    const range = selectionRange();
    if (!range || !scroller) return [];
    const startSide =
      range.startContainer.parentElement?.closest<HTMLElement>('[data-side]')?.dataset.side;
    const side = selectSide ?? startSide ?? null;
    const parts: SelectedPart[] = [];
    for (const element of scroller.querySelectorAll<HTMLElement>('.rows > .row')) {
      const row = rows[indexByKey.get(element.dataset.key!) ?? -1];
      if (!row) continue;
      for (const content of element.querySelectorAll<HTMLElement>('.content')) {
        if (!range.intersectsNode(content)) continue;
        const cell = content.closest<HTMLElement>('.cell')!;
        const line = lineOf(row, cell.dataset.side, side);
        if (!line) continue;
        const { startContainer, startOffset, endContainer, endOffset } = range;
        const start = content.contains(startContainer)
          ? offsetIn(content, startContainer, startOffset)
          : 0;
        const end = content.contains(endContainer)
          ? offsetIn(content, endContainer, endOffset)
          : line.text.length;
        // A selection that ends at the very start of a line doesn't take it.
        if (start >= end && line.text !== '') continue;
        const { kind, oldNo, newNo } = line;
        parts.push({ row, cell, line: { kind, oldNo, newNo, text: line.text.slice(start, end) } });
      }
    }
    return parts;
  }

  /** The selected lines, by file. Rows are in order, so each file's lines come together. */
  function selectedFiles(): SelectedFile[] {
    const byFile: SelectedFile[] = [];
    for (const { row, line } of selectedParts()) {
      const last = byFile.at(-1);
      if (last?.file === row.file) last.lines.push(line);
      else byFile.push({ file: row.file, lines: [line] });
    }
    return byFile;
  }

  /**
   * Puts the button at the right end of the selection's last line, just below
   * it, or above the first line when there's no room below.
   */
  function placeCopyButton() {
    const parts = pointerDown ? [] : selectedParts();
    if (parts.length === 0 || !list || !scroller) {
      copyAt = null;
      return;
    }
    const box = list.getBoundingClientRect();
    const view = scroller.getBoundingClientRect();
    const first = parts[0].cell.getBoundingClientRect();
    const last = parts[parts.length - 1].cell.getBoundingClientRect();
    if (last.bottom < view.top || first.top > view.bottom) {
      copyAt = null;
      return;
    }
    const height = 32;
    const gap = 4;
    const below = last.bottom + gap + height <= view.bottom;
    const top = below ? last.bottom + gap : Math.max(view.top, first.top - height - gap);
    const right = (below ? last : first).right - box.left - 8;
    copyAt = { top: top - box.top, right };
  }

  $effect(() => {
    const onselectionchange = () => {
      copied = false;
      placeCopyButton();
    };
    const onpointerup = () => {
      pointerDown = false;
      placeCopyButton();
    };
    document.addEventListener('selectionchange', onselectionchange);
    window.addEventListener('pointerup', onpointerup);
    return () => {
      document.removeEventListener('selectionchange', onselectionchange);
      window.removeEventListener('pointerup', onpointerup);
      clearTimeout(copiedTimer);
    };
  });

  async function copyForClaude() {
    const files = selectedFiles();
    if (files.length === 0) return;
    const text = claudeReference(files, session.comparison, session.changes?.noMergeBase);
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      return;
    }
    copied = true;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied = false), 1500);
  }

  const vars = $derived.by(() => {
    const { codeFont, fontSize, tabWidth } = settings.value;
    const font = codeFont
      ? `"${codeFont.replaceAll('"', '')}", var(--fontStack-monospace)`
      : 'var(--fontStack-monospace)';
    return [
      `--code-font: ${font}`,
      `--code-size: ${fontSize}px`,
      `--line-height: ${lineHeight}px`,
      `--tab-width: ${tabWidth}`,
      `--num-width: calc(${extent.digits}ch + var(--base-size-16))`,
      `--hunk-gutter-width: calc(${view.diffStyle === 'split' ? 1 : 2} * var(--num-width))`,
      `--code-white-space: ${wrap ? 'pre-wrap' : 'pre'}`,
      `--scroll-x: ${scrollX}px`,
    ].join('; ');
  });
</script>

<div class="diff-list" data-select={selectSide} style={vars} bind:this={list}>
  <!-- Focusable, so the diff scrolls from the keyboard. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="scroller"
    bind:this={scroller}
    bind:clientHeight={viewport}
    bind:clientWidth={width}
    {onscroll}
    {onpointerdown}
    {@attach wheel}
    role="region"
    aria-label="Diff"
    tabindex="0"
  >
    <span class="probe" aria-hidden="true" bind:offsetWidth={probeWidth}>{'0'.repeat(100)}</span>
    {#if sticky}
      <div class="sticky">
        <div
          class="pinned"
          style="height: {HEADER_HEIGHT}px; transform: translateY({sticky.shift}px)"
        >
          <FileHeader
            file={sticky.row.file}
            collapsed={sticky.row.collapsed}
            ontoggle={() => toggle(sticky.row.file.path)}
          />
        </div>
      </div>
    {/if}
    <div class="rows" style="height: {offsets[rows.length] + GAP}px">
      {#each visible as { row, top } (row.key)}
        <div
          class="row"
          data-key={row.key}
          style="top: {top + GAP}px; {measures(row) ? '' : `height: ${fixedHeight(row)}px`}"
          {@attach observe(row)}
        >
          <DiffRow
            {row}
            ontoggle={toggle}
            onexpand={expand}
            onload={(path) => void diffs.load(path, true)}
            highlight={(side, no) => highlights.line(row.file.path, shikiTheme, side, no)}
            marks={(cell) => marks.get(`${row.key}\0${cell}`)}
          />
        </div>
      {/each}
    </div>
  </div>
  {#if view.findOpen}
    <div class="find">
      <FindBar
        bind:query
        count={matches.length}
        current={currentIndex === -1 ? null : currentIndex}
        {searching}
        requests={view.findRequests}
        onnext={() => step(1)}
        onprevious={() => step(-1)}
        onclose={closeFind}
      />
    </div>
  {/if}
  {#if copyAt}
    <div class="copy-for-claude" style="top: {copyAt.top}px; left: {copyAt.right}px">
      <!-- Pressing the button keeps the selection. -->
      <Button size="small" onmousedown={(e) => e.preventDefault()} onclick={copyForClaude}>
        <Icon name={copied ? 'check' : 'copy'} />
        {copied ? 'Copied' : 'Copy for Claude'}
      </Button>
    </div>
  {/if}
  {#if maxScrollX > 0}
    <div class="hscroll" bind:this={hscroll} onscroll={onhscroll}>
      <div style="width: calc(100% + {maxScrollX}px)"></div>
    </div>
  {/if}
</div>

<style>
  .diff-list {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .scroller {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden auto;
    overflow-anchor: none;
  }

  .find {
    position: absolute;
    top: var(--base-size-8);
    right: var(--base-size-24);
    z-index: 2;
  }

  /* Its right edge is at `left`. */
  .copy-for-claude {
    position: absolute;
    z-index: 2;
    transform: translateX(-100%);
  }

  .copy-for-claude :global(.button) {
    box-shadow: var(--shadow-floating-small);
  }

  .probe {
    position: absolute;
    visibility: hidden;
    font-family: var(--code-font);
    font-size: var(--code-size);
    white-space: pre;
  }

  .sticky {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 0;
    margin: 0 var(--base-size-16);
  }

  .pinned {
    background: var(--bgColor-default);
  }

  .pinned :global(.file-header) {
    border-radius: 0;
  }

  .rows {
    position: relative;
    margin: 0 var(--base-size-16);
  }

  .row {
    position: absolute;
    right: 0;
    left: 0;
  }

  /* In split view, a selection doesn't spill into the other side. */
  [data-select='left'] :global([data-side='right']),
  [data-select='right'] :global([data-side='left']) {
    user-select: none;
  }

  /* Tall enough to show an overlay scrollbar (GTK, macOS) on top. */
  .hscroll {
    flex: none;
    min-height: var(--base-size-12);
    overflow: auto hidden;
    border-top: var(--borderWidth-thin) solid var(--borderColor-muted);
  }

  .hscroll > div {
    height: 1px;
  }
</style>
