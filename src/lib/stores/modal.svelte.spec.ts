/**
 * `modal` store — browser-mode runes coverage.
 *
 * Scope:
 *   - Initial state: closed, grid view, no active trader.
 *   - open() flips isOpen=true and resets to grid.
 *   - close() flips isOpen=false.
 *   - showProfile() switches to profile view and sets active trader.
 *   - backToGrid() resets to grid view and clears active trader.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { modal } from './modal.svelte';

beforeEach(() => {
	modal.close();
	modal.backToGrid();
});

describe('modal store', () => {
	it('starts closed in grid view with no active trader', () => {
		expect(modal.activeView).toBe('grid');
		expect(modal.activeTrader).toBeNull();
	});

	it('open() flips isOpen=true and resets to grid', () => {
		// Pre-existing profile state should be wiped on (re)open.
		modal.showProfile('trader-123');
		modal.open();
		expect(modal.isOpen).toBe(true);
		expect(modal.activeView).toBe('grid');
		expect(modal.activeTrader).toBeNull();
	});

	it('close() flips isOpen=false', () => {
		modal.open();
		modal.close();
		expect(modal.isOpen).toBe(false);
	});

	it('showProfile() switches view and stores trader id', () => {
		modal.showProfile('alice');
		expect(modal.activeView).toBe('profile');
		expect(modal.activeTrader).toBe('alice');
	});

	it('backToGrid() resets view and clears trader', () => {
		modal.showProfile('bob');
		modal.backToGrid();
		expect(modal.activeView).toBe('grid');
		expect(modal.activeTrader).toBeNull();
	});
});
