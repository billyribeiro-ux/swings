/**
 * Consent gate — primitives that defer work until all required categories
 * are granted.
 *
 * This file is intentionally free of Svelte runes so third-party scripts loaded
 * via plain `<script>` injection can import it without dragging component
 * context in. It talks to the consent store indirectly, through the
 * `swings:consent:updated` DOM event that the store dispatches on every change.
 *
 * Used by CONSENT-02 (script blocker) to replace the unconditional analytics /
 * marketing loads that exist today.
 */

import {
	CONSENT_EVENT_NAME,
	type ConsentEventDetail
} from '$lib/stores/consent.svelte';
import { consent } from '$lib/stores/consent.svelte';

/**
 * Read the CSP nonce emitted by `src/hooks.server.ts`.
 *
 * Strategy:
 *   1. Server-injected inline scripts in `src/app.html` carry the nonce. We
 *      locate the first `<script nonce="…">` node and read its attribute.
 *   2. If no such node is found (SSR not yet hydrated / test harness) we
 *      return `undefined` and the caller appends the script without a nonce.
 *      That path is still acceptable because the script URL is allowlisted in
 *      the CSP's `script-src` directive; nonce is belt-and-braces.
 */
export function getCspNonce(): string | undefined {
	if (typeof document === 'undefined') return undefined;
	// Prefer reading from the meta tag if present (framework-agnostic channel).
	const meta = document.querySelector<HTMLMetaElement>('meta[name="csp-nonce"]');
	if (meta?.content) return meta.content;
	const scriptWithNonce = document.querySelector<HTMLScriptElement>('script[nonce]');
	if (scriptWithNonce?.nonce) return scriptWithNonce.nonce;
	// Reading the raw attribute covers browsers that don't reflect the nonce
	// property after the initial parse.
	const attr = scriptWithNonce?.getAttribute('nonce');
	return attr ?? undefined;
}

interface GateOptions {
	/** List of category keys — ALL must be granted before `loadScript` resolves. */
	readonly categories: readonly string[];
}

function allGranted(categories: readonly string[]): boolean {
	for (const key of categories) {
		if (!consent.hasCategory(key)) return false;
	}
	return true;
}

/**
 * Load a `<script>` tag once the required categories are granted.
 *
 * Resolution semantics:
 *   - If every required category is already granted, the script is appended
 *     immediately and the returned Promise resolves on `load`.
 *   - Otherwise, the function subscribes to `swings:consent:updated` and
 *     appends+resolves as soon as the grant arrives. The Promise stays
 *     pending forever if the user keeps the category denied — callers that
 *     need a timeout should wrap the call with `Promise.race`.
 *   - De-duplicates: if the same URL is already in the document, resolves
 *     immediately (prevents double-tracking pixels).
 */
export function loadScript(url: string, opts: GateOptions): Promise<void> {
	if (typeof document === 'undefined') {
		return Promise.resolve(); // SSR no-op; hydration re-runs the caller.
	}

	const existing = document.querySelector<HTMLScriptElement>(`script[data-consent-url="${url}"]`);
	if (existing && existing.dataset.consentLoaded === 'true') {
		return Promise.resolve();
	}

	return new Promise<void>((resolve, reject) => {
		const append = () => {
			const script = document.createElement('script');
			script.src = url;
			script.async = true;
			script.crossOrigin = 'anonymous';
			script.dataset.consentUrl = url;
			const nonce = getCspNonce();
			if (nonce) script.nonce = nonce;
			script.onload = () => {
				script.dataset.consentLoaded = 'true';
				resolve();
			};
			script.onerror = () => reject(new Error(`Failed to load gated script: ${url}`));
			document.head.appendChild(script);
		};

		if (allGranted(opts.categories)) {
			append();
			return;
		}

		const handler = (evt: Event) => {
			const detail = (evt as CustomEvent<ConsentEventDetail>).detail;
			if (!detail) return;
			if (allGranted(opts.categories)) {
				window.removeEventListener(CONSENT_EVENT_NAME, handler);
				append();
			}
		};
		window.addEventListener(CONSENT_EVENT_NAME, handler);
	});
}

/**
 * Run `cb` every time the categories transition from "not all granted" to
 * "all granted". Returns an unsubscribe fn. If `cb` itself returns a cleanup
 * function, it is called on the NEXT deny transition (so subscribers can
 * tear down third-party SDKs on withdrawal).
 *
 * Fires once immediately if the grant is already satisfied at call time.
 */
export function onConsent(
	categories: readonly string[],
	cb: () => void | (() => void)
): () => void {
	if (typeof window === 'undefined') return () => undefined;

	let lastlyGranted = false;
	let teardown: (() => void) | void;

	const check = () => {
		const now = allGranted(categories);
		if (now && !lastlyGranted) {
			lastlyGranted = true;
			teardown = cb();
		} else if (!now && lastlyGranted) {
			lastlyGranted = false;
			if (typeof teardown === 'function') {
				teardown();
				teardown = undefined;
			}
		}
	};

	// Fire for the current state.
	check();

	const handler = () => check();
	window.addEventListener(CONSENT_EVENT_NAME, handler);
	return () => {
		window.removeEventListener(CONSENT_EVENT_NAME, handler);
		if (typeof teardown === 'function') teardown();
	};
}
