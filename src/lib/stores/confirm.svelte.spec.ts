/**
 * `confirms` store — browser-mode runes coverage.
 *
 * Scope:
 *   - ask() returns a Promise and pushes to the queue.
 *   - current() reflects the head of the queue.
 *   - resolveCurrent(true) resolves the head's promise with true and shifts.
 *   - Multiple asks queue FIFO; only the head is `current` at a time.
 *   - Variants default to 'info' when not provided.
 *   - Restores focus to the trigger element after resolve.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { confirms, confirmDialog } from './confirm.svelte';

afterEach(() => {
	while (confirms.current) confirms.resolveCurrent(false);
});

describe('confirms store', () => {
	it('ask() pushes onto the queue and exposes head via current', () => {
		const promise = confirms.ask({ title: 'Hi' });
		expect(confirms.current?.title).toBe('Hi');
		// Default variant.
		expect(confirms.current?.variant).toBe('info');
		confirms.resolveCurrent(false);
		return expect(promise).resolves.toBe(false);
	});

	it('resolveCurrent(true) resolves the promise', async () => {
		const p = confirmDialog({ title: 'Confirm?' });
		confirms.resolveCurrent(true);
		await expect(p).resolves.toBe(true);
		expect(confirms.current).toBeNull();
	});

	it('queues multiple asks FIFO', async () => {
		const p1 = confirmDialog({ title: 'first' });
		const p2 = confirmDialog({ title: 'second' });
		expect(confirms.current?.title).toBe('first');
		confirms.resolveCurrent(true);
		await expect(p1).resolves.toBe(true);
		expect(confirms.current?.title).toBe('second');
		confirms.resolveCurrent(false);
		await expect(p2).resolves.toBe(false);
	});

	it('preserves the variant from the request', () => {
		confirms.ask({ title: 'Danger', variant: 'danger' });
		expect(confirms.current?.variant).toBe('danger');
	});

	it('restores focus to the trigger element after resolve', async () => {
		const btn = document.createElement('button');
		btn.textContent = 'trigger';
		document.body.appendChild(btn);
		btn.focus();
		const p = confirmDialog({ title: 'X' });
		// `trigger` is captured at ask() time.
		expect(confirms.current?.trigger).toBe(btn);
		// Move focus elsewhere to prove the restore actually moves it back.
		(document.activeElement as HTMLElement | null)?.blur();
		confirms.resolveCurrent(false);
		await p;
		// queueMicrotask hop.
		await new Promise((r) => setTimeout(r, 0));
		expect(document.activeElement).toBe(btn);
		btn.remove();
	});
});
