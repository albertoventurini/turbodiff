<script lang="ts">
  import { onMount } from 'svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
  import RepoView from './lib/components/RepoView.svelte';
  import Welcome from './lib/components/Welcome.svelte';
  import { setStores } from './lib/stores/context';
  import { RecentStore } from './lib/stores/recent.svelte';
  import { SessionStore } from './lib/stores/session.svelte';
  import { SettingsStore } from './lib/stores/settings.svelte';
  import { ViewStore } from './lib/stores/view.svelte';
  import { applyTheme, resolveMode, SystemMode } from './lib/theme/theme.svelte';

  const isSettingsWindow = new URLSearchParams(window.location.search).get('window') === 'settings';

  const settings = new SettingsStore();
  const session = new SessionStore();
  const recent = new RecentStore();
  const system = new SystemMode();
  setStores({ session, view: new ViewStore(settings.value.diffStyle), settings, recent, system });

  $effect(() => {
    const { theme, appearance } = settings.value;
    applyTheme(document.documentElement, theme, resolveMode(appearance, system.dark));
  });

  onMount(() => {
    const stops = [system.start(), settings.start()];
    if (!isSettingsWindow) stops.push(session.start(), recent.start());
    return () => stops.forEach((stop) => stop());
  });
</script>

{#if isSettingsWindow}
  <!-- The Settings window comes in M8 (PRD §6.8). -->
  <EmptyState icon="info" title="Settings">
    <p>Settings aren't available yet.</p>
  </EmptyState>
{:else if session.repo}
  <RepoView />
{:else}
  <Welcome />
{/if}
