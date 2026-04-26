/**
 * Toaster — browser test coverage.
 *
 * Scope:
 *   1. Mounts the polite/assertive `<ol aria-label="Notifications">` region.
 *   2. `toast.success(...)` adds a `role="status"` row with the title text.
 *   3. `toast.error(...)` is rendered (assertive list).
 *   4. Dismiss button removes the row.
 *   5. Auto-dismiss removes the row after `duration` ms.
 *   6. Action button onClick fires; toast stays mounted (per design).
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import Toaster from './Toaster.svelte';
import { toast } from '$lib/stores/toast.svelte';

afterEach(() => {
	toast.clear();
});

function tick(ms = 30): Promise<void> {
	return new Promise((r) => setTimeout(r, ms));
}

describe('Toaster', () => {
	it('renders the notifications region', async () => {
		render(Toaster);
		const region = page.getByRole('list', { name: /notifications/i });
		await expect.element(region).toBeInTheDocument();
	});

	it('renders a success toast with role=status', async () => {
		render(Toaster);
		toast.success('Saved!', { duration: 0 });
		await tick();
		const row = page.getByRole('status').first();
		await expect.element(row).toBeInTheDocument();
		await expect.element(row).toHaveTextContent(/Saved!/);
	});

	it('renders an error toast with description', async () => {
		render(Toaster);
		toast.error('Oh no', { description: 'Something broke', duration: 0 });
		await tick();
		const row = page.getByRole('status').first();
		await expect.element(row).toHaveTextContent(/Oh no/);
		await expect.element(row).toHaveTextContent(/Something broke/);
	});

	it('dismiss button removes the toast', async () => {
		render(Toaster);
		toast.info('Removable', { duration: 0 });
		await tick();
		expect(toast.items.length).toBe(1);
		const dismiss = page.getByRole('button', { name: /dismiss notification/i }).first();
		await dismiss.click();
		await tick();
		expect(toast.items.length).toBe(0);
	});

	it('auto-dismisses after duration', async () => {
		render(Toaster);
		toast.success('Brief', { duration: 80 });
		await tick(20);
		expect(toast.items.length).toBe(1);
		// rAF-driven timer; give it more than `duration + animation tail`.
		await tick(220);
		expect(toast.items.length).toBe(0);
	});

	it('action button fires its handler without auto-dismissing', async () => {
		render(Toaster);
		const onClick = vi.fn();
		toast.warning('With action', {
			action: { label: 'Undo', onClick },
			duration: 0
		});
		await tick();
		const action = page.getByRole('button', { name: /^undo$/i });
		await action.click();
		expect(onClick).toHaveBeenCalledTimes(1);
		// Per design — clicking the action does not auto-dismiss.
		expect(toast.items.length).toBe(1);
	});
});
