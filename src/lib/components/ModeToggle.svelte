<script lang="ts">
  import type { Mode } from '../types';
  import SegmentedControl from './SegmentedControl.svelte';

  let { value, onchange }: { value: Mode; onchange: (mode: Mode) => void } = $props();

  const OPTIONS: { value: Mode; label: string }[] = [
    { value: 'threeDot', label: 'Three-dot: changes on compare' },
    { value: 'twoDot', label: 'Two-dot: direct comparison' },
  ];

  // The PRD §6.3 explanation, in short.
  const TOOLTIP = [
    'Three-dot (•••): only the changes made on compare since it split off from base.',
    'Two-dot (••): base and compare as they are now, compared directly.',
    'They differ only when base has gained commits since compare split off.',
  ].join('\n');
</script>

<SegmentedControl options={OPTIONS} {value} {onchange} label="Comparison mode" title={TOOLTIP}>
  {#snippet item(option)}
    <span class="dots" aria-hidden="true">{option.value === 'threeDot' ? '•••' : '••'}</span>
  {/snippet}
</SegmentedControl>

<style>
  .dots {
    min-width: var(--base-size-16);
    letter-spacing: var(--base-size-2);
    text-align: center;
  }
</style>
