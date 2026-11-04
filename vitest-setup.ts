import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/svelte';
import { afterEach, vi } from 'vitest';

// Vitest runs without globals, so Testing Library can't register this itself.
afterEach(() => cleanup());

// jsdom has no matchMedia. Tests that need the OS to be dark replace it.
if (!window.matchMedia) {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}
