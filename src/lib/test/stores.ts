// Fresh stores for a component test, passed to `render` as context.
import { storesContext, type Stores } from '../stores/context';
import { RecentStore } from '../stores/recent.svelte';
import { SessionStore } from '../stores/session.svelte';
import { SettingsStore } from '../stores/settings.svelte';
import { ViewStore } from '../stores/view.svelte';

export function makeStores(): Stores & { context: ReturnType<typeof storesContext> } {
  const stores = {
    session: new SessionStore(),
    view: new ViewStore('inline'),
    settings: new SettingsStore(),
    recent: new RecentStore(),
  };
  return { ...stores, context: storesContext(stores) };
}
