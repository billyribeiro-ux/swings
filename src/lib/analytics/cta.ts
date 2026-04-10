import { browser } from '$app/environment';
import { auth } from '$lib/stores/auth.svelte';
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
	if (typeof navigator !== 'undefined' && navigator.doNotTrack === '1') return false;
	if (localStorage.getItem(ANALYTICS_OPT_OUT_KEY) === '1') return false;
	const p = window.location.pathname;
	if (p.startsWith('/admin')) return false;
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

	const apiBase = import.meta.env.VITE_API_URL || 'http://localhost:3001';
	const path =
		pathOverride ?? (typeof window !== 'undefined' ? window.location.pathname + window.location.search : '/');

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

	const headers: Record<string, string> = { 'Content-Type': 'application/json' };
	if (auth.accessToken) {
		headers['Authorization'] = `Bearer ${auth.accessToken}`;
	}

	fetch(`${apiBase}/api/analytics/events`, {
		method: 'POST',
		headers,
		body,
		keepalive: true
	}).catch(() => {});
}

/** Svelte action: log one impression per card when it enters the viewport. */
export function ctaImpression(
	node: HTMLElement,
	params: { ctaId: string; threshold?: number }
): { update: (p: { ctaId: string; threshold?: number }) => void; destroy: () => void } {
	if (!browser) {
		return { update: () => {}, destroy: () => {} };
	}

	let ctaId = params.ctaId;
	let threshold = params.threshold ?? 0.35;
	let done = false;
	let io: IntersectionObserver | null = null;

	function setup() {
		io?.disconnect();
		if (!ctaId || done) return;
		io = new IntersectionObserver(
			(entries) => {
				for (const e of entries) {
					if (e.isIntersecting && !done) {
						done = true;
						trackCtaEvent('impression', ctaId);
						io?.disconnect();
					}
				}
			},
			{ threshold }
		);
		io.observe(node);
	}

	setup();

	return {
		update(p) {
			ctaId = p.ctaId;
			threshold = p.threshold ?? 0.35;
			done = false;
			setup();
		},
		destroy() {
			io?.disconnect();
		}
	};
}
