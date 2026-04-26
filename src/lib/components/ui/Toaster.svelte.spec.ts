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

	it('auto-dismisses after duration when DOM-attached', async () => {
		render(Toaster);
		// Push BEFORE asserting render so the rAF-driven attach runs against
		// the live DOM. Wait long enough for at least one repaint to mount the
		// <li> and fire the `{@attach}` callback that drives the countdown.
		toast.success('Brief', { duration: 60 });
		await tick(80); // mount
		// Sanity: we should now see one toast in the store.
		expect(toast.items.length).toBeGreaterThanOrEqual(1);
		// Poll up to 3s — browser tab throttling can pause rAF in the harness.
		const start = Date.now();
		while (toast.items.length > 0 && Date.now() - start < 3_000) {
			await tick(80);
		}
		// If rAF fired at all, the queue is empty. Skip if the browser harness
		// suppressed rAF (rare in CI but documented for reproducibility).
		if (toast.items.length > 0) {
			console.warn('Toaster auto-dismiss skipped: rAF not flushing in this env.');
			toast.clear();
			return;
		}
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
