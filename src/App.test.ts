import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

vi.mock('./lib/api', () => ({
  appVersion: () => Promise.resolve('1.2.3'),
}));

describe('App', () => {
  it('shows the version reported by the backend', async () => {
    render(App);
    expect(screen.getByRole('heading', { name: 'TurboDiff' })).toBeInTheDocument();
    expect(await screen.findByText('Version 1.2.3')).toBeInTheDocument();
  });
});
