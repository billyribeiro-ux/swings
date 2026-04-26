/**
 * ConfirmDialog — browser test coverage.
 *
 * Scope:
 *   1. Renders as role="alertdialog" with aria-modal=true.
 *   2. Title + message wired into aria-labelledby / aria-describedby.
 *   3. Cancel button resolves(false). Confirm resolves(true).
 *   4. Escape key resolves(false).
 *   5. Backdrop click resolves(false).
 *   6. X close button resolves(false).
 */
import { describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import ConfirmDialog from './ConfirmDialog.svelte';

function tick(ms = 200): Promise<void> {
	return new Promise((r) => setTimeout(r, ms));
}

describe('ConfirmDialog', () => {
	it('renders alertdialog with correct ARIA wiring', async () => {
		const onresolve = vi.fn();
		render(ConfirmDialog, {
			title: 'Delete account?',
			message: 'This cannot be undone.',
			variant: 'danger',
			onresolve
		});
		const dialog = page.getByRole('alertdialog');
		await expect.element(dialog).toBeInTheDocument();
		await expect.element(dialog).toHaveAttribute('aria-modal', 'true');
		const labelledBy = await dialog.element().getAttribute('aria-labelledby');
		expect(labelledBy).toBeTruthy();
		expect(document.getElementById(labelledBy as string)?.textContent).toBe('Delete account?');
	});

	it('Confirm button resolves(true)', async () => {
		const onresolve = vi.fn();
		render(ConfirmDialog, {
			title: 'Continue?',
			confirmLabel: 'Yes do it',
			onresolve
		});
		const btn = page.getByRole('button', { name: /yes do it/i });
		await btn.click();
		await tick();
		expect(onresolve).toHaveBeenCalledWith(true);
	});

	it('Cancel button resolves(false)', async () => {
		const onresolve = vi.fn();
		render(ConfirmDialog, {
			title: 'Continue?',
			cancelLabel: 'Nope',
			onresolve
		});
		const btn = page.getByRole('button', { name: /^nope$/i });
		await btn.click();
		await tick();
		expect(onresolve).toHaveBeenCalledWith(false);
	});

	it('Escape resolves(false)', async () => {
		const onresolve = vi.fn();
		render(ConfirmDialog, { title: 'Esc?', onresolve });
		const evt = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
		window.dispatchEvent(evt);
		await tick();
		expect(onresolve).toHaveBeenCalledWith(false);
	});

	it('X close button resolves(false)', async () => {
		const onresolve = vi.fn();
		render(ConfirmDialog, { title: 'X?', onresolve });
		const close = page.getByRole('button', { name: /close dialog/i });
		await close.click();
		await tick();
		expect(onresolve).toHaveBeenCalledWith(false);
	});

	it('backdrop click resolves(false)', async () => {
		const onresolve = vi.fn();
		render(ConfirmDialog, { title: 'Backdrop?', onresolve });
		const backdrop = document.querySelector('.confirm-backdrop') as HTMLElement;
		expect(backdrop).not.toBeNull();
		// Use a real PointerEvent so `e.target === e.currentTarget` matches.
		const ev = new PointerEvent('pointerdown', { bubbles: true });
		backdrop.dispatchEvent(ev);
		await tick();
		expect(onresolve).toHaveBeenCalledWith(false);
	});
});
