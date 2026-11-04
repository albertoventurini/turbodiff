<script lang="ts">
  import { errorDetail, errorTitle } from '../errors';
  import type { AppError } from '../types';
  import Notice from './Notice.svelte';

  let { error, ondismiss }: { error: AppError; ondismiss: () => void } = $props();

  const detail = $derived(errorDetail(error));
</script>

<Notice tone="danger" {ondismiss}>
  <strong>{errorTitle(error)}</strong>{#if detail}: <code>{detail}</code>{/if}
</Notice>
