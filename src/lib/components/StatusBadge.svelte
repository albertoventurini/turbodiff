<script lang="ts" module>
  import type { Status } from '../types';

  export const STATUS_NAMES: Record<Status, string> = {
    A: 'Added',
    M: 'Modified',
    D: 'Deleted',
    R: 'Renamed',
    T: 'Type changed',
  };
</script>

<script lang="ts">
  let { status }: { status: Status } = $props();
</script>

<span class="badge status-{status}" aria-label={STATUS_NAMES[status]}>{status}</span>

<style>
  .badge {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: var(--base-size-16);
    height: var(--base-size-16);
    border-radius: var(--borderRadius-small);
    font-family: var(--fontStack-monospace);
    font-size: var(--text-caption-size);
    font-weight: var(--base-text-weight-semibold);
    line-height: 1;
  }

  .status-A {
    background: var(--bgColor-success-muted);
    color: var(--fgColor-success);
  }
  .status-M,
  .status-T {
    background: var(--bgColor-attention-muted);
    color: var(--fgColor-attention);
  }
  .status-D {
    background: var(--bgColor-danger-muted);
    color: var(--fgColor-danger);
  }
  .status-R {
    background: var(--bgColor-accent-muted);
    color: var(--fgColor-accent);
  }
</style>
