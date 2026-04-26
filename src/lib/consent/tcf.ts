/**
 * IAB TCF v2.2 publisher-only shim.
 *
 * The full TCF v2.2 protocol is a base64 bitstring carrying vendor-list
 * consent, purpose consent, legitimate-interest flags, special-feature
 * opt-ins, and publisher TC. Running the full spec requires the `@iabtcf/core`
 * library plus a daily-refreshed Global Vendor List.
 *
 * CONSENT-04 ships a narrowly-scoped implementation: publisher-only consent
 * (no vendor list, no LI flags). We generate a valid-looking base64 TC string
 * from the current category map and expose the `__tcfapi` postMessage
 * protocol so CMP-aware downstream tags (Google's stub.js, Meta Pixel, etc.)
 * can read it. When we adopt the full GVL later, the only thing that changes
 * is `buildTcString` — the `__tcfapi` plumbing stays.
 *
 * TODO: wire `@iabtcf/core` + scheduled GVL refresh. See CONSENT-04 v2 in
 *       docs/archive/AUDIT_PHASE3_PLAN.md §3.
 */

import { CONSENT_EVENT_NAME, type ConsentEventDetail } from '$lib/stores/consent.svelte';

/** TCF v2.2 purpose map — a subset we actually need for publisher-only consent. */
const TCF_PURPOSE_MAP: Readonly<Record<string, readonly number[]>> = {
	// 1 — Store and/or access information on a device
	functional: [1],
	// 2..6 + 9 — Ad-adjacent purposes (select/personalize/measure ads + content).
	marketing: [2, 3, 4, 7, 9],
	// 7..10 — Measurement + research (analytics).
	analytics: [7, 8, 10],
	// 5, 6 — Personalization (select personalised content, etc.).
	personalization: [5, 6]
};

/** Aggregate a category map into the set of purposes the subject has granted. */
function derivePurposesGranted(categories: Readonly<Record<string, boolean>>): ReadonlySet<number> {
	const granted = new Set<number>();
	for (const [key, enabled] of Object.entries(categories)) {
		if (!enabled) continue;
		const purposes = TCF_PURPOSE_MAP[key];
		if (!purposes) continue;
		for (const p of purposes) granted.add(p);
	}
	return granted;
}

/**
 * Build a TCF v2.2-compatible string. The output is NOT a valid GVL-aware
 * string yet — it carries the publisher-consent bits in a format the
 * `__tcfapi` shim can serve, but downstream servers verifying against the
 * IAB schema will reject it. Interim measure until `@iabtcf/core` lands.
 *
 * The string format here is `swings-pub-v1.<base64(JSON payload)>.<iso8601>`.
 * Consumers should treat it as opaque.
 */
export function buildTcString(categories: Readonly<Record<string, boolean>>): string {
	const purposes = [...derivePurposesGranted(categories)].sort((a, b) => a - b);
	const payload = {
		v: 2,
		p: purposes,
		// Publisher-only consent — no vendor restrictions encoded.
		vendors: [],
		// April 2026 spec: CMP id 0xFFFF is the reserved "non-TCF" range; we
		// occupy it intentionally until the site registers with the IAB.
		cmp_id: 0xffff,
		cmp_version: 1,
		// Minimal spec-required fields.
		tcf_policy_version: 5,
		publisher_cc: 'US'
	};
	const json = JSON.stringify(payload);
	const b64 = encodeBase64Url(json);
	// Issued-at is recorded as the integer epoch-second count — keeps the
	// third segment free of `.` so consumers can split on `.` safely.
	const ts = Math.floor(Date.now() / 1000).toString(36);
	return `swings-pub-v1.${b64}.${ts}`;
}

function encodeBase64Url(input: string): string {
	if (typeof btoa === 'function') {
		// `btoa` only accepts ASCII; we UTF-8 encode first.
		const bytes = new TextEncoder().encode(input);
		let binary = '';
		for (const b of bytes) binary += String.fromCharCode(b);
		const b64 = btoa(binary);
		return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
	}
	// Node / SSR fallback — `Buffer` is only available under Node.
	const globalWithBuffer = globalThis as typeof globalThis & {
		Buffer?: { from(data: string, encoding: string): { toString(encoding: string): string } };
	};
	if (typeof globalWithBuffer.Buffer !== 'undefined') {
		return globalWithBuffer.Buffer.from(input, 'utf-8')
			.toString('base64')
			.replace(/\+/g, '-')
			.replace(/\//g, '_')
			.replace(/=+$/, '');
	}
	// Last-ditch: return the raw JSON so tests still exercise the shape.
	return input;
}

// ── __tcfapi shim ────────────────────────────────────────────────────────

/** Common shape of the __tcfapi callback. Accept `any`-ish data to match spec. */
type TcfApiCallback = (data: unknown, success: boolean) => void;

type TcfCommand = 'ping' | 'getTCData' | 'addEventListener' | 'removeEventListener';

interface TcfPingReturn {
	readonly gdprApplies: boolean | null;
	readonly cmpLoaded: boolean;
	readonly cmpStatus: 'stub' | 'loading' | 'loaded' | 'error';
	readonly displayStatus: 'visible' | 'hidden' | 'disabled';
	readonly apiVersion: '2.2';
	readonly cmpVersion: number;
	readonly cmpId: number;
	readonly gvlVersion: number | null;
	readonly tcfPolicyVersion: number;
}

interface TcfData {
	readonly tcString: string;
	readonly tcfPolicyVersion: number;
	readonly cmpId: number;
	readonly cmpVersion: number;
	readonly gdprApplies: boolean | null;
	readonly eventStatus: 'tcloaded' | 'cmpuistatus' | 'useractioncomplete';
	readonly cmpStatus: 'loaded';
	readonly listenerId?: number;
	readonly purpose: { readonly consents: Readonly<Record<number, boolean>> };
	readonly publisher: { readonly consents: Readonly<Record<number, boolean>> };
}

interface TcfApi {
	(command: TcfCommand, version: 2, callback: TcfApiCallback, parameter?: unknown): void;
}

let listenerSeq = 0;
const listeners = new Map<number, TcfApiCallback>();
let currentTcString = '';
let currentPurposes: ReadonlySet<number> = new Set();

function buildTcData(eventStatus: TcfData['eventStatus']): TcfData {
	const purposeBits: Record<number, boolean> = {};
	for (let i = 1; i <= 10; i++) {
		purposeBits[i] = currentPurposes.has(i);
	}
	return {
		tcString: currentTcString,
		tcfPolicyVersion: 5,
		cmpId: 0xffff,
		cmpVersion: 1,
		gdprApplies: null,
		eventStatus,
		cmpStatus: 'loaded',
		purpose: { consents: purposeBits },
		publisher: { consents: purposeBits }
	};
}

function buildPing(): TcfPingReturn {
	return {
		gdprApplies: null,
		cmpLoaded: true,
		cmpStatus: 'loaded',
		displayStatus: 'hidden',
		apiVersion: '2.2',
		cmpVersion: 1,
		cmpId: 0xffff,
		gvlVersion: null,
		tcfPolicyVersion: 5
	};
}

let tcfApiInstalled = false;

/**
 * Install the `__tcfapi` shim on `window`. Wires an event listener to the
 * consent store so the TC string is refreshed on every mutation. Idempotent.
 */
export function installTcfApi(currentCategories?: Readonly<Record<string, boolean>>): void {
	if (typeof window === 'undefined') return;
	if (tcfApiInstalled) return;
	tcfApiInstalled = true;

	if (currentCategories) {
		currentTcString = buildTcString(currentCategories);
		currentPurposes = derivePurposesGranted(currentCategories);
	}

	const api: TcfApi = (command, _version, callback, parameter) => {
		switch (command) {
			case 'ping':
				callback(buildPing(), true);
				return;
			case 'getTCData': {
				const data = buildTcData('tcloaded');
				callback(data, true);
				return;
			}
			case 'addEventListener': {
				listenerSeq += 1;
				const id = listenerSeq;
				listeners.set(id, callback);
				const data: TcfData = { ...buildTcData('tcloaded'), listenerId: id };
				callback(data, true);
				return;
			}
			case 'removeEventListener': {
				const removeId = typeof parameter === 'number' ? parameter : -1;
				const removed = listeners.delete(removeId);
				callback(removed, removed);
				return;
			}
			default: {
				callback(null, false);
			}
		}
	};

	const win = window as Window & { __tcfapi?: TcfApi };
	win.__tcfapi = api;

	window.addEventListener(CONSENT_EVENT_NAME, (evt) => {
		const detail = (evt as CustomEvent<ConsentEventDetail>).detail;
		if (!detail) return;
		currentTcString = buildTcString(detail.state.categories);
		currentPurposes = derivePurposesGranted(detail.state.categories);
		const data = buildTcData('useractioncomplete');
		for (const [id, cb] of listeners) {
			try {
				cb({ ...data, listenerId: id }, true);
			} catch {
				// A single broken listener must never take down the rest.
			}
		}
	});
}

/** Test-only reset. */
export function __resetTcfApiForTests(): void {
	tcfApiInstalled = false;
	listeners.clear();
	listenerSeq = 0;
	currentTcString = '';
	currentPurposes = new Set();
}
