/**
 * `toast` store (newer ToastStore singleton) — browser-mode runes coverage.
 *
 * The sister store at `./toasts.svelte.ts` is covered separately. This file
 * exercises the `toast.{success,error,warning,info}` API + queue eviction.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { toast } from './toast.svelte';

afterEach(() => {
	toast.clear();
});

describe('toast store', () => {
	it('success() appends an item with variant=success', () => {
		const id = toast.success('Saved', { duration: 0 });
		expect(typeof id).toBe('string');
		expect(toast.items.length).toBe(1);
		expect(toast.items[0].variant).toBe('success');
		expect(toast.items[0].title).toBe('Saved');
	});

	it('error() defaults duration to 6000ms', () => {
		const id = toast.error('Boom');
		const item = toast.items.find((t) => t.id === id);
		expect(item?.duration).toBe(6000);
	});

	it('respects an explicit duration override', () => {
		const id = toast.info('Brief', { duration: 100 });
		expect(toast.items.find((t) => t.id === id)?.duration).toBe(100);
	});

	it('dismiss() removes a row by id', () => {
		const id = toast.warning('Watch', { duration: 0 });
		expect(toast.dismiss(id)).toBe(true);
		expect(toast.items.length).toBe(0);
	});

	it('dismiss() returns false on unknown id', () => {
		expect(toast.dismiss('does-not-exist')).toBe(false);
	});

	it('clear() empties the list', () => {
		toast.success('a', { duration: 0 });
		toast.error('b', { duration: 0 });
		toast.clear();
		expect(toast.items.length).toBe(0);
	});

	it('caps queue at 5 visible items, evicting oldest first', () => {
		for (let i = 0; i < 7; i += 1) toast.info(`t${i}`, { duration: 0 });
		expect(toast.items.length).toBe(5);
		// Two oldest should have been shifted off.
		expect(toast.items[0].title).toBe('t2');
		expect(toast.items[4].title).toBe('t6');
	});

	it('action option is preserved on the item', () => {
		const action = { label: 'Undo', onClick: () => undefined };
		const id = toast.warning('With action', { action, duration: 0 });
		expect(toast.items.find((t) => t.id === id)?.action).toEqual(action);
	});
});
