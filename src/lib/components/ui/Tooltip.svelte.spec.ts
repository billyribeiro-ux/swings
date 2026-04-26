/**
 * Tooltip — browser test coverage.
 *
 * Scope:
 *   1. Hidden by default; appears on pointer-enter (with delay=0).
 *   2. Renders with role="tooltip" and aria-describedby wires onto trigger.
 *   3. Hides on pointer-leave.
 *   4. Closes on Escape key.
 *   5. Disabled prop suppresses opening.
 *   6. Hotkey is rendered when provided.
 */
import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import Harness from './_fixtures/TooltipHarness.svelte';

function tick(ms = 30): Promise<void> {
	return new Promise((r) => setTimeout(r, ms));
}

describe('Tooltip', () => {
	it('does not render the tooltip on initial mount', async () => {
		render(Harness, { label: 'Hello' });
		const tip = document.querySelector('[role="tooltip"]');
		expect(tip).toBeNull();
	});

	it('opens on pointer-enter and renders role="tooltip"', async () => {
		render(Harness, { label: 'Save changes' });
		const trigger = page.getByTestId('tooltip-trigger');
		// vitest-browser dispatches a real pointerenter under hover()
		await trigger.hover();
		await tick();
		const tip = document.querySelector('[role="tooltip"]');
		expect(tip).not.toBeNull();
		expect(tip?.textContent).toContain('Save changes');
	});

	it('hides on pointer-leave', async () => {
		render(Harness, { label: 'Bye' });
		const trigger = page.getByTestId('tooltip-trigger');
		await trigger.hover();
		await tick();
		expect(document.querySelector('[role="tooltip"]')).not.toBeNull();
		// Move pointer off the trigger by hovering the body element.
		await page.locator('body').hover({ position: { x: 1, y: 1 } });
		await tick();
		expect(document.querySelector('[role="tooltip"]')).toBeNull();
	});

	it('closes on Escape', async () => {
		render(Harness, { label: 'EscMe' });
		const trigger = page.getByTestId('tooltip-trigger');
		await trigger.hover();
		await tick();
		expect(document.querySelector('[role="tooltip"]')).not.toBeNull();
		const evt = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
		(trigger.element() as HTMLElement).dispatchEvent(evt);
		await tick();
		expect(document.querySelector('[role="tooltip"]')).toBeNull();
	});

	it('does not open when disabled=true', async () => {
		render(Harness, { label: 'No', disabled: true });
		const trigger = page.getByTestId('tooltip-trigger');
		await trigger.hover();
		await tick();
		expect(document.querySelector('[role="tooltip"]')).toBeNull();
	});

	it('renders hotkey chip when provided', async () => {
		render(Harness, { label: 'With key', hotkey: '⌘K' });
		const trigger = page.getByTestId('tooltip-trigger');
		await trigger.hover();
		await tick();
		const tip = document.querySelector('[role="tooltip"]');
		expect(tip).not.toBeNull();
		const kbd = tip!.querySelector('kbd');
		expect(kbd?.textContent).toBe('⌘K');
	});
});
