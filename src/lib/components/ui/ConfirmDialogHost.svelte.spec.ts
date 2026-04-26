/**
 * ConfirmDialogHost — browser test coverage.
 *
 * Scope:
 *   1. Renders nothing when the queue is empty.
 *   2. `confirms.ask(...)` mounts the dialog with the given title.
 *   3. Resolving via the host wires through the store's promise.
 *   4. Multiple asks are queued FIFO; only the head renders.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import ConfirmDialogHost from './ConfirmDialogHost.svelte';
import { confirms, confirmDialog } from '$lib/stores/confirm.svelte';

afterEach(() => {
	// Drop anything left in the queue so tests don't leak state.
	while (confirms.current) confirms.resolveCurrent(false);
});

function tick(ms = 50): Promise<void> {
	return new Promise((r) => setTimeout(r, ms));
}

describe('ConfirmDialogHost', () => {
	it('renders nothing when queue is empty', async () => {
		render(ConfirmDialogHost);
		await tick();
		expect(document.querySelector('[role="alertdialog"]')).toBeNull();
	});

	it('mounts the dialog when confirms.ask is invoked', async () => {
		render(ConfirmDialogHost);
		const promise = confirmDialog({ title: 'Hosted prompt' });
		await tick();
		const dialog = page.getByRole('alertdialog');
		await expect.element(dialog).toBeInTheDocument();
		await expect.element(dialog).toHaveTextContent(/Hosted prompt/);

		// Cleanup — resolve to drain the queue and let the test finish.
		confirms.resolveCurrent(false);
		await expect(promise).resolves.toBe(false);
	});

	it('resolves with true when confirm is clicked', async () => {
		render(ConfirmDialogHost);
		const promise = confirmDialog({ title: 'Sure?', confirmLabel: 'Yep' });
		await tick();
		const yes = page.getByRole('button', { name: /^yep$/i });
		await yes.click();
		const result = await promise;
		expect(result).toBe(true);
	});

	it('queues multiple asks FIFO; only head is rendered', async () => {
		render(ConfirmDialogHost);
		const p1 = confirmDialog({ title: 'First' });
		const p2 = confirmDialog({ title: 'Second' });
		await tick();
		// Only the head should render at a time.
		expect(document.querySelectorAll('[role="alertdialog"]').length).toBe(1);
		expect(document.body.textContent).toContain('First');
		expect(document.body.textContent).not.toContain('Second');

		confirms.resolveCurrent(true);
		await expect(p1).resolves.toBe(true);
		await tick();
		// Now Second should be rendered.
		expect(document.body.textContent).toContain('Second');
		confirms.resolveCurrent(false);
		await expect(p2).resolves.toBe(false);
	});
});
