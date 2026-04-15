<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { browser } from '$app/environment';
	import { auth } from '$lib/stores/auth.svelte';
	import { ANALYTICS_OPT_OUT_KEY, ANALYTICS_SESSION_KEY } from '$lib/analytics/constants';
	import { getPublicApiBase } from '$lib/api/publicApiBase';

	function getSessionId(): string {
		if (!browser) return '';
		let id = sessionStorage.getItem(ANALYTICS_SESSION_KEY);
		if (!id) {
			id = crypto.randomUUID();
			sessionStorage.setItem(ANALYTICS_SESSION_KEY, id);
		}
		return id;
	}

	function shouldTrack(path: string): boolean {
		if (!browser) return false;
		if (navigator.doNotTrack === '1') return false;
		if (localStorage.getItem(ANALYTICS_OPT_OUT_KEY) === '1') return false;
		if (path.startsWith('/admin')) return false;
		return true;
	}

	let lastSentPath = '';
	let lastSentAt = 0;

	function sendPageView() {
		if (!browser) return;
		const path = page.url.pathname + page.url.search;
		if (!shouldTrack(page.url.pathname)) return;
		// Prevent accidental double-emits from rapid same-route transitions.
		const now = Date.now();
		if (path === lastSentPath && now - lastSentAt < 1500) return;
		lastSentPath = path;
		lastSentAt = now;

		const apiBase = getPublicApiBase();
		const body = JSON.stringify({
			session_id: getSessionId(),
			events: [
				{
					event_type: 'page_view',
					path: path || '/',
					referrer: document.referrer ? document.referrer.slice(0, 2048) : null,
					metadata: {
						user_status: auth.isAuthenticated ? (auth.isAdmin ? 'admin' : 'member') : 'logged_out'
					}
				}
			]
		});

		fetch(`${apiBase}/api/analytics/events`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body,
			keepalive: true
		}).catch(() => {});
	}

	// `afterNavigate` fires on initial load too — no need for an additional `onMount`,
	// which would double-count the first page view.
	afterNavigate(sendPageView);
</script>
