/**
 * Google Consent Mode v2 bridge.
 *
 * CONSENT-04 implements the April-2026 GCM v2 protocol: a denied-by-default
 * consent frame at mount time, then an `update` frame on every change of the
 * consent store's category map. The signals are pushed to `window.dataLayer`
 * exactly as `gtag('consent', …)` would; downstream Google tags (Tag Manager,
 * GA4, Google Ads) pick them up automatically.
 *
 * Signal mapping (our categories → GCM signals):
 *   - analytics       → analytics_storage
 *   - marketing       → ad_storage, ad_user_data, ad_personalization
 *   - functional      → functional_storage
 *   - personalization → personalization_storage
 *   - necessary       → (not mapped; necessary is always-on and not a GCM signal)
 *
 * `security_storage` is deliberately left out of the update frame: it tracks
 * security / fraud-prevention reads which are always allowed under GDPR
 * legitimate-interest and CCPA exceptions, so we leave whatever value the
 * site-default (usually `granted`) carries.
 *
 * GPC interaction: the consent store auto-denies every non-necessary category
 * when `navigator.globalPrivacyControl === true`. `installGcmBridge()` runs
 * `pushGcmUpdate` on every consent event — including the synthetic `denied`
 * event the store emits during hydration — so GPC propagates into GCM without
 * any explicit wiring.
 */

import { CONSENT_EVENT_NAME, type ConsentEventDetail } from '$lib/stores/consent.svelte';

/**
 * Minimal `dataLayer` frame shape. `window.dataLayer` is an array of tuples
 * (`['consent','default',config]`, `['event','page_view']`, …); GCM only uses
 * the `'consent'` verb.
 */
type GcmSignal = 'granted' | 'denied';

export interface GcmDefaultConfig {
	readonly ad_storage: GcmSignal;
	readonly analytics_storage: GcmSignal;
	readonly ad_user_data: GcmSignal;
	readonly ad_personalization: GcmSignal;
	readonly functional_storage?: GcmSignal;
	readonly personalization_storage?: GcmSignal;
	readonly security_storage?: GcmSignal;
	readonly wait_for_update: number;
}

export interface GcmUpdateConfig {
	readonly ad_storage?: GcmSignal;
	readonly analytics_storage?: GcmSignal;
	readonly ad_user_data?: GcmSignal;
	readonly ad_personalization?: GcmSignal;
	readonly functional_storage?: GcmSignal;
	readonly personalization_storage?: GcmSignal;
}

// `unknown[]` is the historical GTM contract — entries can be tuples of any
// shape. We push only consent frames ourselves but tolerate existing entries.
type DataLayer = unknown[];

interface DataLayerWindow {
	dataLayer?: DataLayer;
}

/**
 * Push GCM v2 defaults to `window.dataLayer`. Every non-essential signal is
 * `denied` at mount time — the April-2026 GCM v2 spec requires this so the
 * site gates tag firing until the user makes a decision. `wait_for_update`
 * gives downstream tags a 500 ms grace window to receive the update frame
 * before they fire with the default (denied) state.
 *
 * Idempotent via the `__swings_gcm_defaults_pushed` flag on `window` — calling
 * twice is a no-op, so SSR→hydration double-mounts don't double-push.
 */
export function pushGcmDefaults(): void {
	if (typeof window === 'undefined') return;
	const win = window as Window & DataLayerWindow & { __swings_gcm_defaults_pushed?: boolean };
	if (win.__swings_gcm_defaults_pushed === true) return;
	win.dataLayer = win.dataLayer ?? [];
	const defaults: GcmDefaultConfig = {
		ad_storage: 'denied',
		analytics_storage: 'denied',
		ad_user_data: 'denied',
		ad_personalization: 'denied',
		functional_storage: 'denied',
		personalization_storage: 'denied',
		wait_for_update: 500
	};
	win.dataLayer.push(['consent', 'default', defaults]);
	win.__swings_gcm_defaults_pushed = true;
}

/**
 * Map our internal category map to the GCM v2 signal shape and push an
 * `update` frame. Pure function otherwise — the only side effect is the
 * single `dataLayer.push` call at the end.
 *
 * Exported from the module so the unit test can call it directly without
 * touching the bridge / event-listener plumbing.
 */
export function pushGcmUpdate(categories: Readonly<Record<string, boolean>>): void {
	if (typeof window === 'undefined') return;
	const win = window as Window & DataLayerWindow;
	win.dataLayer = win.dataLayer ?? [];
	const update = mapCategoriesToGcm(categories);
	win.dataLayer.push(['consent', 'update', update]);
}

/**
 * Category → GCM signal mapping. Exported so the unit test can compare the
 * shape without a DOM. Keys with no corresponding category in the input map
 * are omitted from the frame — GCM treats missing signals as "unchanged",
 * which is exactly what we want when a category doesn't appear.
 */
export function mapCategoriesToGcm(categories: Readonly<Record<string, boolean>>): GcmUpdateConfig {
	const toSignal = (v: boolean | undefined): GcmSignal => (v === true ? 'granted' : 'denied');
	const out: Record<string, GcmSignal> = {};

	if ('analytics' in categories) {
		out.analytics_storage = toSignal(categories.analytics);
	}
	if ('marketing' in categories) {
		const signal = toSignal(categories.marketing);
		out.ad_storage = signal;
		out.ad_user_data = signal;
		out.ad_personalization = signal;
	}
	if ('functional' in categories) {
		out.functional_storage = toSignal(categories.functional);
	}
	if ('personalization' in categories) {
		out.personalization_storage = toSignal(categories.personalization);
	}

	return out as GcmUpdateConfig;
}

let bridgeInstalled = false;

/**
 * Install the end-to-end GCM v2 bridge. Idempotent — safe to call from a
 * `$effect` that re-runs on hot-reload. Subscribes to the consent-store event
 * and pushes an `update` frame on every mutation.
 *
 * The bridge intentionally pushes the current store state once at install
 * time (synchronously) so late-mounting consumers see the already-granted
 * categories immediately, not just on the next user interaction.
 */
export function installGcmBridge(currentCategories?: Readonly<Record<string, boolean>>): void {
	if (typeof window === 'undefined') return;
	if (bridgeInstalled) return;
	bridgeInstalled = true;

	pushGcmDefaults();

	if (currentCategories) {
		pushGcmUpdate(currentCategories);
	}

	window.addEventListener(CONSENT_EVENT_NAME, (evt) => {
		const detail = (evt as CustomEvent<ConsentEventDetail>).detail;
		if (!detail) return;
		pushGcmUpdate(detail.state.categories);
	});
}

/** Test-only reset — clears the install + defaults flags so the bridge can be reinstalled. */
export function __resetGcmBridgeForTests(): void {
	bridgeInstalled = false;
	if (typeof window !== 'undefined') {
		const win = window as Window & { __swings_gcm_defaults_pushed?: boolean };
		delete win.__swings_gcm_defaults_pushed;
	}
}
