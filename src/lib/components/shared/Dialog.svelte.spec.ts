/**
 * Dialog — browser test coverage.
 *
 * Focus: the four contractual a11y behaviours of the component.
 *   1. Correct ARIA semantics (role=dialog, aria-modal, aria-labelledby, aria-describedby).
 *   2. Focus trap: initial focus lands on the first focusable element.
 *   3. Escape closes (when closeOnEscape === true).
 *   4. Backdrop click closes (when closeOnBackdrop === true).
 */
import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import DialogHarness from './_fixtures/DialogHarness.svelte';

function pressKey(key: string) {
	const evt = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
	document.dispatchEvent(evt);
}

describe('Dialog', () => {
	it('renders with dialog semantics', async () => {
		render(DialogHarness, { initialOpen: true });
		const dialog = page.getByRole('dialog');
		await expect.element(dialog).toBeInTheDocument();
		await expect.element(dialog).toHaveAttribute('aria-modal', 'true');
		await expect.element(dialog).toHaveAttribute('aria-labelledby');
		await expect.element(dialog).toHaveAttribute('aria-describedby');
	});

	it('labels the dialog via aria-labelledby', async () => {
		render(DialogHarness, { initialOpen: true, title: 'Unique Title 789' });
		const dialog = page.getByRole('dialog');
		await expect.element(dialog).toBeInTheDocument();
		const labelledBy = await dialog.element().getAttribute('aria-labelledby');
		expect(labelledBy).toBeTruthy();
		const label = document.getElementById(labelledBy as string);
		expect(label?.textContent).toBe('Unique Title 789');
	});

	it('places initial focus on the first focusable child', async () => {
		render(DialogHarness, { initialOpen: true });
		// requestAnimationFrame inside the focusTrap attachment defers focus.
		await new Promise((r) => requestAnimationFrame(() => r(null)));
		await new Promise((r) => setTimeout(r, 30));
		const first = document.querySelector<HTMLElement>('[data-testid="first-focusable"]');
		expect(document.activeElement).toBe(first);
	});

	it('closes on Escape', async () => {
		render(DialogHarness, { initialOpen: true });
		await new Promise((r) => setTimeout(r, 30));
		pressKey('Escape');
		await new Promise((r) => setTimeout(r, 30));
		const dialog = document.querySelector('[role="dialog"]');
		expect(dialog).toBeNull();
	});

	it('does NOT close on Escape when closeOnEscape=false', async () => {
		render(DialogHarness, { initialOpen: true, closeOnEscape: false });
		await new Promise((r) => setTimeout(r, 30));
		pressKey('Escape');
		await new Promise((r) => setTimeout(r, 30));
		const dialog = document.querySelector('[role="dialog"]');
		expect(dialog).not.toBeNull();
	});

	it('closes on backdrop click when closeOnBackdrop=true', async () => {
		render(DialogHarness, { initialOpen: true });
		await new Promise((r) => setTimeout(r, 30));
		const backdrop = document.querySelector<HTMLElement>('[role="presentation"]');
		expect(backdrop).not.toBeNull();
		backdrop!.click();
		await new Promise((r) => setTimeout(r, 30));
		const dialog = document.querySelector('[role="dialog"]');
		expect(dialog).toBeNull();
	});

	it('does NOT close on backdrop click when closeOnBackdrop=false', async () => {
		render(DialogHarness, { initialOpen: true, closeOnBackdrop: false });
		await new Promise((r) => setTimeout(r, 30));
		const backdrop = document.querySelector<HTMLElement>('[role="presentation"]');
		backdrop!.click();
		await new Promise((r) => setTimeout(r, 30));
		const dialog = document.querySelector('[role="dialog"]');
		expect(dialog).not.toBeNull();
	});
});
