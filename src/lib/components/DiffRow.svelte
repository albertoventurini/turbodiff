<script lang="ts">
  import {
    spans,
    type Cell,
    type ExpandDirection,
    type Message,
    type Row,
    type Token,
  } from '../diffRows';
  import type { LineKind, Side } from '../types';
  import Button from './Button.svelte';
  import FileHeader from './FileHeader.svelte';
  import Icon from './Icon.svelte';

  let {
    row,
    ontoggle,
    onexpand,
    onload,
    highlight = () => undefined,
  }: {
    row: Row;
    ontoggle: (path: string) => void;
    onexpand: (row: Extract<Row, { kind: 'hunk' }>, direction: ExpandDirection) => void;
    onload: (path: string) => void;
    /** The highlighting of line `no` on one side of the row's file, if any. */
    highlight?: (side: Side, no: number) => Token[] | undefined;
  } = $props();

  /**
   * The highlighting of a line: deleted lines are on the old side, added and
   * unchanged ones on the new side (unchanged lines show the new text).
   */
  function tokens(kind: LineKind, oldNo: number | null, newNo: number | null) {
    if (kind === 'del') return oldNo === null ? undefined : highlight('old', oldNo);
    return newNo === null ? undefined : highlight('new', newNo);
  }

  const MARKERS: Record<LineKind, string> = { ctx: ' ', add: '+', del: '−' };

  const EXPANDERS: Record<
    ExpandDirection,
    { icon: 'fold-up' | 'fold-down' | 'unfold'; label: string }
  > = {
    up: { icon: 'fold-up', label: 'Expand up' },
    down: { icon: 'fold-down', label: 'Expand down' },
    all: { icon: 'unfold', label: 'Expand all' },
  };

  function size(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ['KB', 'MB', 'GB'];
    let value = bytes / 1024;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit++;
    }
    return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
  }

  function short(sha: string): string | null {
    return /^0+$/.test(sha) ? null : sha.slice(0, 7);
  }
</script>

{#snippet code(
  kind: LineKind,
  text: string,
  words: Cell['words'],
  noNewline: boolean,
  highlighted: Token[] | undefined,
)}
  <span class="code" data-marker={MARKERS[kind]}>
    <span class="clip"
      ><span class="content"
        >{#each spans(text, words, highlighted) as part, i (i)}{#if part.changed}<span
              class="word"
              style={part.style}>{part.text}</span
            >{:else if part.style}<span style={part.style}>{part.text}</span
            >{:else}{part.text}{/if}{/each}{#if noNewline}<span
            class="eof"
            title="No newline at end of file"
            aria-label="No newline at end of file"><Icon name="no-entry" /></span
          >{/if}</span
      ></span
    >
  </span>
{/snippet}

{#snippet half(cell: Cell | null, side: 'left' | 'right', newNo: number | null)}
  {#if cell}
    <span class="num {cell.kind}" data-no={cell.no}></span>
    <span class="cell {cell.kind}" data-side={side}>
      {@render code(
        cell.kind,
        cell.text,
        cell.words,
        cell.noNewline,
        tokens(cell.kind, cell.no, newNo),
      )}
    </span>
  {:else}
    <span class="num filler"></span>
    <span class="cell filler" data-side={side}></span>
  {/if}
{/snippet}

{#snippet message(message: Message)}
  {#if message.kind === 'tooLarge'}
    <span>Large diff not shown.</span>
    <Button size="small" onclick={() => onload(row.file.path)}>Load diff</Button>
  {:else if message.kind === 'binary'}
    {@const { oldSize, newSize } = message}
    <span>Binary file not shown.</span>
    {#if oldSize !== null && newSize !== null}
      <span class="detail">{size(oldSize)} → {size(newSize)}</span>
    {:else if oldSize !== null || newSize !== null}
      <span class="detail">{size((oldSize ?? newSize)!)}</span>
    {/if}
  {:else if message.kind === 'whitespaceOnly'}
    <span>Whitespace-only changes.</span>
  {:else if message.kind === 'modeChange'}
    <span>File mode changed <code>{message.from}</code> → <code>{message.to}</code></span>
  {:else if message.kind === 'submodule'}
    {@const from = short(message.from)}
    {@const to = short(message.to)}
    <span>
      {#if from && to}
        Submodule commit <code>{from}</code> → <code>{to}</code>
      {:else if to}
        Submodule added at commit <code>{to}</code>
      {:else}
        Submodule removed, was at commit <code>{from}</code>
      {/if}
    </span>
  {:else}
    <span>{message.text}</span>
  {/if}
{/snippet}

{#if row.kind === 'header'}
  <FileHeader file={row.file} collapsed={row.collapsed} ontoggle={() => ontoggle(row.file.path)} />
{:else if row.kind === 'hunk'}
  <div class="body hunk">
    <span class="expanders">
      {#each row.directions as direction (direction)}
        <button
          type="button"
          aria-label={EXPANDERS[direction].label}
          title={EXPANDERS[direction].label}
          onclick={() => onexpand(row, direction)}
        >
          <Icon name={EXPANDERS[direction].icon} />
        </button>
      {/each}
    </span>
    <span class="hunk-text">{row.header ?? ''}</span>
  </div>
{:else if row.kind === 'line'}
  {@const line = row.line}
  <div class="body line {line.kind}">
    <span class="num {line.kind}" data-no={line.oldNo ?? ''}></span>
    <span class="num {line.kind}" data-no={line.newNo ?? ''}></span>
    <span class="cell {line.kind}">
      {@render code(
        line.kind,
        line.text,
        line.words,
        line.noNewline,
        tokens(line.kind, line.oldNo, line.newNo),
      )}
    </span>
  </div>
{:else if row.kind === 'split'}
  <div class="body split">
    <!-- An unchanged line's left cell has the old number, but the new text. -->
    {@render half(row.left, 'left', row.right?.no ?? null)}
    {@render half(row.right, 'right', row.right?.no ?? null)}
  </div>
{:else if row.kind === 'message'}
  <div class="body message">{@render message(row.message)}</div>
{:else if row.kind === 'placeholder'}
  <div class="body placeholder"><span class="loading">Loading diff…</span></div>
{:else if row.closed}
  <div class="end"></div>
{/if}

<style>
  /* A row inside a file's card: the card's side borders. */
  .body {
    height: 100%;
    border: solid var(--borderColor-default);
    border-width: 0 var(--borderWidth-thin);
    background: var(--bgColor-default);
  }

  /* The bottom of the card, below its last row. */
  .end {
    height: var(--borderRadius-medium);
    border: var(--borderWidth-thin) solid var(--borderColor-default);
    border-top: 0;
    border-radius: 0 0 var(--borderRadius-medium) var(--borderRadius-medium);
    background: var(--bgColor-default);
  }

  .line {
    display: grid;
    grid-template-columns: var(--num-width) var(--num-width) minmax(0, 1fr);
  }

  .split {
    display: grid;
    grid-template-columns: var(--num-width) minmax(0, 1fr) var(--num-width) minmax(0, 1fr);
  }

  .num {
    padding: 0 var(--base-size-8);
    color: var(--fgColor-muted);
    font-family: var(--code-font);
    font-size: var(--code-size);
    line-height: var(--line-height);
    text-align: right;
    user-select: none;
  }

  /* Line numbers and markers come from CSS, so they aren't copied. */
  .num::before {
    content: attr(data-no);
  }

  .split .num:nth-of-type(3),
  .split .filler:nth-of-type(3) {
    border-left: var(--borderWidth-thin) solid var(--borderColor-default);
  }

  .cell {
    min-width: 0;
    overflow: hidden;
  }

  .code {
    display: block;
    position: relative;
    padding: 0 var(--base-size-8) 0 var(--base-size-24);
    font-family: var(--code-font);
    font-size: var(--code-size);
    line-height: var(--line-height);
    white-space: var(--code-white-space);
    overflow-wrap: anywhere;
    tab-size: var(--tab-width);
  }

  .code::before {
    content: attr(data-marker);
    position: absolute;
    left: var(--base-size-8);
    user-select: none;
  }

  /* Lines scrolled sideways don't cover the marker. */
  .clip {
    display: block;
    overflow: hidden;
  }

  .content {
    display: inline-block;
    min-width: 100%;
    transform: translateX(calc(-1 * var(--scroll-x)));
  }

  .eof {
    display: inline-flex;
    margin-left: var(--base-size-4);
    color: var(--fgColor-danger);
    vertical-align: middle;
    user-select: none;
  }

  .num.add {
    background: var(--diffBlob-additionNum-bgColor);
    color: var(--diffBlob-additionNum-fgColor);
  }
  .cell.add {
    background: var(--diffBlob-additionLine-bgColor);
    color: var(--diffBlob-additionLine-fgColor);
  }
  .add .word {
    border-radius: var(--borderRadius-small);
    background: var(--diffBlob-additionWord-bgColor);
    color: var(--diffBlob-additionWord-fgColor);
  }

  .num.del {
    background: var(--diffBlob-deletionNum-bgColor);
    color: var(--diffBlob-deletionNum-fgColor);
  }
  .cell.del {
    background: var(--diffBlob-deletionLine-bgColor);
    color: var(--diffBlob-deletionLine-fgColor);
  }
  .del .word {
    border-radius: var(--borderRadius-small);
    background: var(--diffBlob-deletionWord-bgColor);
    color: var(--diffBlob-deletionWord-fgColor);
  }

  .num.filler {
    background: var(--diffBlob-emptyNum-bgColor);
  }
  .cell.filler {
    background: var(--diffBlob-emptyLine-bgColor);
  }

  .hunk {
    display: flex;
    align-items: stretch;
    background: var(--diffBlob-hunkLine-bgColor);
    color: var(--diffBlob-hunkLine-fgColor);
    font-family: var(--code-font);
    font-size: var(--code-size);
    user-select: none;
  }

  .expanders {
    display: flex;
    flex: none;
    width: var(--hunk-gutter-width);
    background: var(--diffBlob-hunkNum-bgColor-rest);
  }

  .expanders button {
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    background: none;
    color: var(--diffBlob-expander-iconColor);
    cursor: pointer;
  }

  .expanders button:hover {
    background: var(--diffBlob-hunkNum-bgColor-hover);
    color: var(--diffBlob-hunkNum-fgColor-hover);
  }

  .hunk-text {
    display: flex;
    align-items: center;
    min-width: 0;
    padding: 0 var(--base-size-8);
    overflow: hidden;
    white-space: pre;
  }

  .message {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: var(--base-size-8);
    padding: 0 var(--base-size-16);
    color: var(--fgColor-muted);
    text-align: center;
    user-select: none;
  }

  .message code {
    font-family: var(--fontStack-monospace);
    font-size: 85%;
  }

  .detail {
    font-family: var(--fontStack-monospace);
    font-size: var(--text-body-size-small);
  }

  .placeholder {
    display: flex;
    justify-content: center;
    padding-top: var(--base-size-16);
  }

  /* Shown only if loading takes a moment, so a quick load doesn't flicker. */
  .loading {
    color: var(--fgColor-muted);
    opacity: 0;
    animation: appear 0s 400ms forwards;
  }

  @keyframes appear {
    to {
      opacity: 1;
    }
  }
</style>
