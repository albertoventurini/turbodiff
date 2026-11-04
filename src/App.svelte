<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import RepoView from './lib/components/RepoView.svelte';
  import SettingsWindow from './lib/components/SettingsWindow.svelte';
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
  const view = new ViewStore(settings.value.diffStyle);
  setStores({ session, view, settings, recent, system });

  $effect(() => {
    const { theme, appearance } = settings.value;
    applyTheme(document.documentElement, theme, resolveMode(appearance, system.dark));
  });

  // A new default diff style applies to every open window (PRD §6.8). Other
  // settings changes leave the toolbar's choice alone.
  const defaultStyle = $derived(settings.value.diffStyle);
  let appliedStyle = untrack(() => defaultStyle);
  $effect(() => {
    if (defaultStyle === appliedStyle) return;
    appliedStyle = defaultStyle;
    view.diffStyle = defaultStyle;
  });

  onMount(() => {
    const stops = [system.start(), settings.start()];
    if (!isSettingsWindow) stops.push(session.start(), recent.start());
    return () => stops.forEach((stop) => stop());
  });
</script>

{#if isSettingsWindow}
  <SettingsWindow />
{:else if session.repo}
  <RepoView />
{:else}
  <Welcome />
{/if}
