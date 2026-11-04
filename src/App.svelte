<script lang="ts">
  import { appVersion } from './lib/api';

  const version = appVersion();
</script>

<main>
  <h1>TurboDiff</h1>
  {#await version then v}
    <p>Version {v}</p>
  {/await}
</main>
