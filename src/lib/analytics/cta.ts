import { browser } from '$app/environment';
import { untrack } from 'svelte';
import type { Attachment } from 'svelte/attachments';
import { getPublicApiBase } from '$lib/api/publicApiBase';
import { ANALYTICS_OPT_OUT_KEY, ANALYTICS_SESSION_KEY } from './constants';

function getSessionId(): string {
	if (!browser) return '';
	let id = sessionStorage.getItem(ANALYTICS_SESSION_KEY);
	if (!id) {
		id = crypto.randomUUID();
		sessionStorage.setItem(ANALYTICS_SESSION_KEY, id);
	}
	return id;
}

function allowTracking(): boolean {
	if (!browser) return false;
	if (navigator.doNotTrack === '1') return false;
	if (localStorage.getItem(ANALYTICS_OPT_OUT_KEY) === '1') return false;
	if (window.location.pathname.startsWith('/admin')) return false;
	return true;
}

/**
 * Fire a single CTA impression or click (for CTR in admin analytics).
 * Requires `metadata.cta_id` on the server; pairs with impressions for the same id.
 */
export function trackCtaEvent(
	type: 'impression' | 'click',
	ctaId: string,
	pathOverride?: string
): void {
	if (!allowTracking() || !ctaId) return;

	const apiBase = getPublicApiBase();
	const path =
		pathOverride ??
		(typeof window !== 'undefined' ? window.location.pathname + window.location.search : '/');

	const body = JSON.stringify({
		session_id: getSessionId(),
		events: [
			{
				event_type: type,
				path: path || '/',
				referrer:
					typeof document !== 'undefined' && document.referrer
						? document.referrer.slice(0, 2048)
						: null,
				metadata: { cta_id: ctaId }
			}
		]
	});

	// BFF (Phase 1.3): authentication rides on the httpOnly `swings_access`
	// cookie now — `credentials: 'include'` ships it automatically. We no
	// longer attach `Authorization: Bearer ...` because the SPA cannot read
	// the token. Anonymous CTA events still send (the analytics endpoint
	// uses `OptionalAuthUser`).
	const headers: Record<string, string> = { 'Content-Type': 'application/json' };

	fetch(`${apiBase}/api/analytics/events`, {
		method: 'POST',
		credentials: 'include',
		headers,
		body,
		keepalive: true
	}).catch(() => {});
}

/**
 * Svelte 5.29+ attachment factory: log one impression per card when it enters the viewport.
 *
 * Usage: `<div {@attach ctaImpression({ ctaId: 'pricing_monthly' })}>...`
 *
 * `untrack` isolates the analytics call from the surrounding tracking context so that
 * any reactive state read inside `trackCtaEvent` cannot accidentally re-trigger the
 * factory and re-create the IntersectionObserver.
 */
export function ctaImpression(params: {
	ctaId: string;
	threshold?: number;
}): Attachment<HTMLElement> {
	return (node) => {
		if (!browser) return;
		const ctaId = params.ctaId;
		if (!ctaId) return;
		const threshold = params.threshold ?? 0.35;
		let done = false;

		const io = new IntersectionObserver(
			(entries) => {
				for (const e of entries) {
					if (e.isIntersecting && !done) {
						done = true;
						untrack(() => trackCtaEvent('impression', ctaId));
						io.disconnect();
					}
				}
			},
			{ threshold }
		);
		io.observe(node);

		return () => io.disconnect();
	};
}
