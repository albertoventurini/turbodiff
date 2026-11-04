import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../test/fakeApi';
import { changeSet } from '../test/fakeApi';
import { defaultComparison } from '../stores/session.svelte';
import { makeStores } from '../test/stores';
import Toolbar from './Toolbar.svelte';

vi.mock('../api', () => import('../test/fakeApi'));

beforeEach(() => vi.resetAllMocks());

function setup() {
  const stores = makeStores();
  stores.session.repo = { root: '/work/app', name: 'app', head: { branch: 'main', commit: 'a' } };
  render(Toolbar, { context: stores.context });
  return stores;
}

describe('Toolbar', () => {
  it('switches between inline and split', async () => {
    const { view } = setup();
    const group = screen.getByRole('group', { name: 'Diff style' });
    expect(screen.getByRole('button', { name: 'Inline' })).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(screen.getByRole('button', { name: 'Split' }));
    expect(view.diffStyle).toBe('split');
    expect(group.querySelector('[aria-pressed="true"]')).toHaveTextContent('Split');
  });

  it('hides whitespace through the comparison', async () => {
    const { session } = setup();
    api.setComparison.mockResolvedValue(changeSet({ generation: 1 }));
    const toggle = screen.getByRole('button', { name: 'Hide whitespace' });
    expect(toggle).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(toggle);
    expect(api.setComparison).toHaveBeenCalledWith(defaultComparison(true));
    expect(session.comparison.ignoreWhitespace).toBe(true);
    expect(toggle).toHaveAttribute('aria-pressed', 'true');
  });
});
