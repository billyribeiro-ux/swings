/**
 * Consent store — runes-backed single source of truth for category grants.
 *
 * Integrates with:
 *   - `$lib/api/consent` for stub record calls today, live CONSENT-01 tomorrow.
 *   - `$lib/consent/gate` for script-loading gated on category grants.
 *   - `$lib/components/consent/ConsentBanner.svelte` for the user-facing surface.
 *
 * Persistence:
 *   localStorage["swings_consent_v1"] — JSON envelope (see `ConsentEnvelope`).
 *   Bumping the storage key (`_v2` etc.) is the migration story; we never
 *   silently mutate the envelope shape under consumers.
 *
 * GPC (Global Privacy Control):
 *   On first load with `navigator.globalPrivacyControl === true` AND no stored
 *   decision, we auto-seed `hasDecided=true, categories={necessary:true,
 *   rest:false}`, flag `gpc=true`, emit a `swings:consent:updated` event, and
 *   submit a `denied` record call. The banner never renders in this case —
 *   that is the whole point of GPC. California (CCPA/CPRA), Colorado (CPA),
 *   and Connecticut (CTDPA) require us to honor it as a valid opt-out.
 *
 * DOM event:
 *   Every state change dispatches a `CustomEvent<ConsentEventDetail>` named
 *   `swings:consent:updated` on `window`. Consumers that can't (or shouldn't)
 *   import the store directly (third-party scripts loaded via `consent/gate`)
 *   can subscribe to the event instead.
 */

import { browser } from '$app/environment';
import { DEFAULT_CATEGORIES, recordConsent, type ConsentAction } from '$lib/api/consent';

/** Storage key — versioned so we can ship migrations by bumping the suffix. */
export const CONSENT_STORAGE_KEY = 'swings_consent_v1';
/** Custom event dispatched on every state change. Payload is `ConsentEventDetail`. */
export const CONSENT_EVENT_NAME = 'swings:consent:updated';
/** Envelope version — bump when the persisted shape changes incompatibly. */
export const CONSENT_ENVELOPE_VERSION = 1;

export interface ConsentEnvelope {
	readonly version: number;
	readonly bannerVersion: number;
	readonly policyVersion: number;
	readonly decidedAt: string;
	readonly gpc: boolean;
	readonly categories: Readonly<Record<string, boolean>>;
}

/**
 * Public snapshot shape. `readonly` modifiers are intentionally omitted — the
 * store mutates its own `$state<ConsentState>` through the API methods, and
 * marking these fields as `readonly` would make the proxy un-assignable in
 * strict TypeScript. Consumers should treat this as an immutable view via the
 * `consent.decision` derived property.
 */
export interface ConsentState {
	version: number;
	categories: Record<string, boolean>;
	hasDecided: boolean;
	gpc: boolean;
	decidedAt: string | null;
}

export interface ConsentDecision {
	readonly hasDecided: boolean;
	readonly categories: Readonly<Record<string, boolean>>;
	readonly gpc: boolean;
}

export interface ConsentEventDetail {
	readonly action: ConsentAction;
	readonly state: ConsentState;
}

/**
 * Build the default category map (necessary on, rest off).
 * Exported so callers can derive a "fresh slate" baseline without bleeding
 * store internals.
 */
export function defaultCategoryMap(
	categories: ReadonlyArray<{
		key: string;
		required: boolean;
		defaultEnabled: boolean;
	}> = DEFAULT_CATEGORIES
): Record<string, boolean> {
	const map: Record<string, boolean> = {};
	for (const cat of categories) {
		map[cat.key] = cat.required ? true : cat.defaultEnabled;
	}
	return map;
}

function isBrowser(): boolean {
	// Guarded separately from SvelteKit's `browser` so tests can mock `window`
	// without pretending to be inside a SvelteKit runtime.
	return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function hasGpcSignal(): boolean {
	if (!isBrowser()) return false;
	const nav = window.navigator as Navigator & { globalPrivacyControl?: boolean };
	return nav.globalPrivacyControl === true;
}

/**
 * ISO-8601 timestamp helper. Wrapped in a function so the `Date` instance is
 * neither stored nor mutated — satisfies the `svelte/prefer-svelte-reactivity`
 * lint rule without needing a SvelteDate wrapper (we only ever need the
 * serialised string for audit logging).
 */
function nowIso(): string {
	return new Date(Date.now()).toISOString();
}

function readEnvelope(storage: Storage | undefined): ConsentEnvelope | null {
	if (!storage) return null;
	try {
		const raw = storage.getItem(CONSENT_STORAGE_KEY);
		if (!raw) return null;
		const parsed = JSON.parse(raw) as unknown;
		if (!parsed || typeof parsed !== 'object') return null;
		const candidate = parsed as Partial<ConsentEnvelope>;
		if (
			typeof candidate.version !== 'number' ||
			typeof candidate.bannerVersion !== 'number' ||
			typeof candidate.policyVersion !== 'number' ||
			typeof candidate.decidedAt !== 'string' ||
			typeof candidate.gpc !== 'boolean' ||
			!candidate.categories ||
			typeof candidate.categories !== 'object'
		) {
			return null;
		}
		return candidate as ConsentEnvelope;
	} catch {
		return null;
	}
}

function emitEvent(action: ConsentAction, state: ConsentState): void {
	if (!isBrowser()) return;
	try {
		const detail: ConsentEventDetail = { action, state };
		window.dispatchEvent(new CustomEvent(CONSENT_EVENT_NAME, { detail }));
	} catch {
		// Swallow — the store must never throw just because an event listener did.
	}
}

/**
 * Runes-backed consent store. A class so tests can spin up isolated instances
 * without leaking into the module-level singleton.
 */
export class ConsentStore {
	/** Active category → granted map. Mutated through the store's API only. */
	readonly state = $state<ConsentState>({
		version: CONSENT_ENVELOPE_VERSION,
		categories: defaultCategoryMap(),
		hasDecided: false,
		gpc: false,
		decidedAt: null
	});

	/** Derived: has the subject recorded any decision at all? */
	readonly hasDecided = $derived(this.state.hasDecided);

	/** Derived: machine-readable decision for consumers needing all three bits. */
	readonly decision = $derived<ConsentDecision>({
		hasDecided: this.state.hasDecided,
		categories: this.state.categories,
		gpc: this.state.gpc
	});

	private readonly storage: Storage | undefined;
	private hydrated = false;

	constructor(storage: Storage | undefined = isBrowser() ? window.localStorage : undefined) {
		this.storage = storage;
		this.hydrate();
		this.installPersistenceEffect();
	}

	/** Has the user granted a specific category? */
	hasCategory = (key: string): boolean => {
		return this.state.categories[key] === true;
	};

	/** Grant a list of categories (others left untouched). */
	grant = (categories: readonly string[]): void => {
		this.mutate('granted', (draft) => {
			for (const key of categories) draft[key] = true;
		});
	};

	/** Deny a list of categories (others left untouched). `necessary` is protected. */
	deny = (categories: readonly string[]): void => {
		this.mutate('denied', (draft) => {
			for (const key of categories) {
				if (key === 'necessary') continue;
				draft[key] = false;
			}
		});
	};

	/** Turn every known category on. */
	acceptAll = (): void => {
		this.mutate('granted', (draft) => {
			for (const key of Object.keys(draft)) draft[key] = true;
		});
	};

	/** Turn every non-necessary category off. `necessary` stays on. */
	rejectAll = (): void => {
		this.mutate('denied', (draft) => {
			for (const key of Object.keys(draft)) {
				draft[key] = key === 'necessary';
			}
		});
	};

	/** Set a single category to a specific value. `necessary` is always granted. */
	updateCategory = (key: string, enabled: boolean): void => {
		this.mutate('updated', (draft) => {
			draft[key] = key === 'necessary' ? true : enabled;
		});
	};

	/**
	 * Clear persistence and reset to defaults so the banner shows again.
	 * Emits a `revoked` event and posts a `revoked` record call so audit log
	 * captures the user's right-to-withdraw action.
	 */
	revokeAll = (): void => {
		if (this.storage) {
			try {
				this.storage.removeItem(CONSENT_STORAGE_KEY);
			} catch {
				// Safari private mode etc. — state still resets in memory.
			}
		}
		this.state.categories = defaultCategoryMap();
		this.state.hasDecided = false;
		this.state.gpc = hasGpcSignal();
		this.state.decidedAt = null;
		emitEvent('revoked', this.snapshot());
		void recordConsent('revoked', this.state.categories);
	};

	/** Manually apply an envelope — used in tests and server-seed scenarios. */
	hydrateFrom = (envelope: ConsentEnvelope): void => {
		this.state.categories = { ...envelope.categories };
		this.state.hasDecided = true;
		this.state.gpc = envelope.gpc;
		this.state.decidedAt = envelope.decidedAt;
	};

	/** Plain-object snapshot — useful for event payloads & test assertions. */
	snapshot = (): ConsentState => ({
		version: this.state.version,
		categories: { ...this.state.categories },
		hasDecided: this.state.hasDecided,
		gpc: this.state.gpc,
		decidedAt: this.state.decidedAt
	});

	/**
	 * Core mutation helper. Builds a draft of the categories map, applies the
	 * mutator, commits the result, marks hasDecided, emits the event, and
	 * records the consent call. Every public mutator funnels through here so
	 * there is exactly one place to audit for correctness.
	 */
	private mutate(action: ConsentAction, mutator: (draft: Record<string, boolean>) => void): void {
		const draft: Record<string, boolean> = { ...this.state.categories };
		mutator(draft);
		// `necessary` is a non-negotiable invariant.
		if (Object.prototype.hasOwnProperty.call(draft, 'necessary')) {
			draft.necessary = true;
		}
		this.state.categories = draft;
		this.state.hasDecided = true;
		// Every mutation restamps `decidedAt`; the audit row downstream is what
		// distinguishes "first decision" from "subsequent update".
		this.state.decidedAt = nowIso();
		emitEvent(action, this.snapshot());
		void recordConsent(action, draft);
	}

	/**
	 * On first construction: read localStorage, apply stored decision if any,
	 * otherwise check GPC and auto-seed if the signal is present.
	 */
	private hydrate(): void {
		const envelope = readEnvelope(this.storage);
		if (envelope) {
			this.hydrateFrom(envelope);
			this.hydrated = true;
			return;
		}

		if (hasGpcSignal()) {
			const draft = defaultCategoryMap();
			// GPC is a directive to deny non-essential processing; `necessary`
			// stays on because it is not opt-out-able under GDPR/CCPA anyway.
			for (const key of Object.keys(draft)) draft[key] = key === 'necessary';
			this.state.categories = draft;
			this.state.hasDecided = true;
			this.state.gpc = true;
			this.state.decidedAt = nowIso();
			emitEvent('denied', this.snapshot());
			void recordConsent('denied', draft);
		}
		this.hydrated = true;
	}

	/**
	 * Persistence effect — runs in component-binding scope only (i.e. when the
	 * store is instantiated from within a Svelte component tree). `$effect.root`
	 * gives us an owned scope with explicit cleanup, so this works both at
	 * module-import time and inside a test harness without leaking.
	 */
	private installPersistenceEffect(): void {
		if (!this.storage) return;
		// `$effect.root` returns a cleanup fn we intentionally never call — the
		// store lives for the lifetime of the page. Using the root form also
		// sidesteps the "orphaned $effect" warning Svelte emits otherwise.
		$effect.root(() => {
			$effect(() => {
				// Read every field so the effect is re-registered on change.
				const { categories, hasDecided, gpc, decidedAt } = this.state;
				if (!this.hydrated) return;
				if (!hasDecided || !decidedAt) return;
				const envelope: ConsentEnvelope = {
					version: CONSENT_ENVELOPE_VERSION,
					bannerVersion: 1,
					policyVersion: 1,
					decidedAt,
					gpc,
					categories: { ...categories }
				};
				try {
					this.storage?.setItem(CONSENT_STORAGE_KEY, JSON.stringify(envelope));
				} catch {
					// Safari private mode, quota exceeded, etc. — we tried our best.
				}
			});
		});
	}
}

/**
 * Shared default instance. Most consumers want this. Instantiate directly with
 * `new ConsentStore(undefined)` for tests that need an isolated fixture.
 */
export const consent: ConsentStore = browser ? new ConsentStore() : new ConsentStore(undefined);
