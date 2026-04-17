/**
 * Toast store — unit coverage for the PE7 shared primitive store.
 *
 * Note: `.svelte.spec.ts` convention steers this file to the Vitest browser
 * config. That's intentional — runes only execute inside a component
 * compilation context, so the store must be exercised in a browser project.
 */
import { describe, expect, it } from 'vitest';
import { ToastStore } from './toasts.svelte';

describe('ToastStore', () => {
	it('push() returns an id and appends the item', () => {
		const store = new ToastStore();
		const id = store.push({ title: 'Hi' });
		expect(typeof id).toBe('string');
		expect(store.items.length).toBe(1);
		expect(store.items[0].title).toBe('Hi');
	});

	it('defaults kind to "info" and duration to 5000', () => {
		const store = new ToastStore();
		store.push({ title: 'T' });
		expect(store.items[0].kind).toBe('info');
		expect(store.items[0].duration).toBe(5000);
	});

	it('remove() drops a matching id and returns true', () => {
		const store = new ToastStore();
		const id = store.push({ title: 'Drop' });
		expect(store.remove(id)).toBe(true);
		expect(store.items.length).toBe(0);
	});

	it('remove() returns false on unknown id', () => {
		const store = new ToastStore();
		expect(store.remove('nope')).toBe(false);
	});

	it('clear() empties the list', () => {
		const store = new ToastStore();
		store.push({ title: 'a' });
		store.push({ title: 'b' });
		store.clear();
		expect(store.items.length).toBe(0);
	});

	it('caps the number of toasts to maxToasts', () => {
		const store = new ToastStore(3);
		for (let i = 0; i < 5; i += 1) store.push({ title: `t${i}` });
		expect(store.items.length).toBe(3);
		// The three most recent survive; oldest two are evicted.
		expect(store.items.map((t) => t.title)).toEqual(['t2', 't3', 't4']);
	});
});
