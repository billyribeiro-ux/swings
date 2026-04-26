/**
 * Consent store — unit coverage.
 *
 * Runes execute only inside a browser compilation context, so this spec uses
 * the `.svelte.spec.ts` suffix to steer Vitest to the browser config.
 *
 * Scope:
 *   - acceptAll flips every category on.
 *   - rejectAll preserves necessary, denies everything else.
 *   - revokeAll clears persistence and hasDecided.
 *   - GPC (navigator.globalPrivacyControl === true) auto-denies marketing +
 *     personalization WITHOUT prompting, and flags gpc=true.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	CONSENT_EVENT_NAME,
	CONSENT_STORAGE_KEY,
	ConsentStore,
	defaultCategoryMap,
	type ConsentEventDetail
} from './consent.svelte';

type GpcNavigator = Navigator & { globalPrivacyControl?: boolean };

function createMemoryStorage(): Storage {
	const map = new Map<string, string>();
	const storage: Storage = {
		get length(): number {
			return map.size;
		},
		clear(): void {
			map.clear();
		},
		getItem(key: string): string | null {
			return map.has(key) ? (map.get(key) as string) : null;
		},
		key(index: number): string | null {
			return Array.from(map.keys())[index] ?? null;
		},
		removeItem(key: string): void {
			map.delete(key);
		},
		setItem(key: string, value: string): void {
			map.set(key, value);
		}
	};
	return storage;
}

describe('ConsentStore', () => {
	let storage: Storage;

	beforeEach(() => {
		storage = createMemoryStorage();
		// Ensure GPC is not set from a previous test.
		(window.navigator as GpcNavigator).globalPrivacyControl = false;
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('starts with hasDecided=false and defaults applied', () => {
		const store = new ConsentStore(storage);
		expect(store.state.hasDecided).toBe(false);
		expect(store.state.gpc).toBe(false);
		expect(store.state.categories).toEqual(defaultCategoryMap());
		expect(store.state.categories.necessary).toBe(true);
	});

	it('acceptAll() flips every category on and marks decided', () => {
		const store = new ConsentStore(storage);
		store.acceptAll();
		expect(store.state.hasDecided).toBe(true);
		for (const v of Object.values(store.state.categories)) {
			expect(v).toBe(true);
		}
	});

	it('rejectAll() preserves necessary and denies the rest', () => {
		const store = new ConsentStore(storage);
		store.rejectAll();
		expect(store.state.hasDecided).toBe(true);
		expect(store.state.categories.necessary).toBe(true);
		expect(store.state.categories.analytics).toBe(false);
		expect(store.state.categories.marketing).toBe(false);
		expect(store.state.categories.personalization).toBe(false);
		expect(store.state.categories.functional).toBe(false);
	});

	it('grant() only flips the listed categories on', () => {
		const store = new ConsentStore(storage);
		store.grant(['analytics']);
		expect(store.state.categories.analytics).toBe(true);
		expect(store.state.categories.marketing).toBe(false);
	});

	it('deny() ignores an attempt to disable necessary', () => {
		const store = new ConsentStore(storage);
		store.deny(['necessary', 'analytics']);
		expect(store.state.categories.necessary).toBe(true);
		expect(store.state.categories.analytics).toBe(false);
	});

	it('updateCategory() for necessary always coerces to true', () => {
		const store = new ConsentStore(storage);
		store.updateCategory('necessary', false);
		expect(store.state.categories.necessary).toBe(true);
	});

	it('revokeAll() clears storage, hasDecided, and decidedAt', () => {
		const store = new ConsentStore(storage);
		store.acceptAll();
		expect(storage.getItem(CONSENT_STORAGE_KEY)).not.toBeNull();
		store.revokeAll();
		expect(store.state.hasDecided).toBe(false);
		expect(store.state.decidedAt).toBeNull();
		expect(storage.getItem(CONSENT_STORAGE_KEY)).toBeNull();
	});

	it('hasCategory() reports the current grant', () => {
		const store = new ConsentStore(storage);
		expect(store.hasCategory('analytics')).toBe(false);
		store.grant(['analytics']);
		expect(store.hasCategory('analytics')).toBe(true);
	});

	it('emits swings:consent:updated on every mutation', async () => {
		const store = new ConsentStore(storage);
		const events: ConsentEventDetail[] = [];
		const handler = (evt: Event) => {
			events.push((evt as CustomEvent<ConsentEventDetail>).detail);
		};
		window.addEventListener(CONSENT_EVENT_NAME, handler);
		try {
			store.acceptAll();
			store.rejectAll();
			store.revokeAll();
			expect(events.length).toBe(3);
			expect(events[0]!.action).toBe('granted');
			expect(events[1]!.action).toBe('denied');
			expect(events[2]!.action).toBe('revoked');
		} finally {
			window.removeEventListener(CONSENT_EVENT_NAME, handler);
		}
	});

	it('GPC signal auto-denies non-essential categories without prompting', () => {
		(window.navigator as GpcNavigator).globalPrivacyControl = true;
		const store = new ConsentStore(storage);
		expect(store.state.gpc).toBe(true);
		expect(store.state.hasDecided).toBe(true);
		expect(store.state.categories.necessary).toBe(true);
		expect(store.state.categories.analytics).toBe(false);
		expect(store.state.categories.marketing).toBe(false);
		expect(store.state.categories.personalization).toBe(false);
	});

	it('hydrates from a stored envelope', () => {
		const now = new Date().toISOString();
		storage.setItem(
			CONSENT_STORAGE_KEY,
			JSON.stringify({
				version: 1,
				bannerVersion: 1,
				policyVersion: 1,
				decidedAt: now,
				gpc: false,
				categories: {
					necessary: true,
					functional: false,
					analytics: true,
					marketing: false,
					personalization: false
				}
			})
		);
		const store = new ConsentStore(storage);
		expect(store.state.hasDecided).toBe(true);
		expect(store.state.categories.analytics).toBe(true);
		expect(store.state.decidedAt).toBe(now);
	});

	it('ignores a malformed stored envelope and starts fresh', () => {
		storage.setItem(CONSENT_STORAGE_KEY, 'not json');
		const store = new ConsentStore(storage);
		expect(store.state.hasDecided).toBe(false);
	});
});
