<script lang="ts">
	import { onMount } from 'svelte';
	import { afterNavigate } from '$app/navigation';
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import { api, ApiError } from '$lib/api/client';
	import { auth } from '$lib/stores/auth.svelte';
	import type { Popup } from '$lib/api/types';
	import PopupRenderer from './PopupRenderer.svelte';

	const STORAGE_KEY_PREFIX = 'swings_popup_';
	const SESSION_KEY_PREFIX = 'swings_popup_session_';

	let activePopups = $state<Popup[]>([]);
	let visiblePopupIds = $state<Set<string>>(new Set());
	let cleanupFns: Array<() => void> = [];
	let popupRequestSeq = 0;
	let popupFetchFailures = 0;
	let popupFetchPauseUntil = 0;

	const currentPath = $derived(page.url.pathname);

	function getDevice(): string {
		if (!browser) return 'desktop';
		const w = window.innerWidth;
		if (w < 768) return 'mobile';
		if (w < 1024) return 'tablet';
		return 'desktop';
	}

	function getUserStatus(): string {
		if (!auth.isAuthenticated) return 'logged_out';
		if (auth.isAdmin) return 'logged_in';
		return 'member';
	}

	function hasBeenShown(popupId: string, frequency: string): boolean {
		if (!browser) return false;
		if (frequency === 'every_time') return false;
		if (frequency === 'once_ever') {
			return localStorage.getItem(`${STORAGE_KEY_PREFIX}${popupId}`) === '1';
		}
		if (frequency === 'once_per_session') {
			return sessionStorage.getItem(`${SESSION_KEY_PREFIX}${popupId}`) === '1';
		}
		return false;
	}

	function markAsShown(popupId: string, frequency: string): void {
		if (!browser) return;
		if (frequency === 'once_ever') {
			localStorage.setItem(`${STORAGE_KEY_PREFIX}${popupId}`, '1');
		}
		if (frequency === 'once_per_session') {
			sessionStorage.setItem(`${SESSION_KEY_PREFIX}${popupId}`, '1');
		}
	}

	function showPopup(popup: Popup): void {
		if (visiblePopupIds.has(popup.id)) return;
		if (hasBeenShown(popup.id, popup.display_frequency)) return;

		visiblePopupIds = new Set([...visiblePopupIds, popup.id]);
		markAsShown(popup.id, popup.display_frequency);
		trackEvent(popup.id, 'impression');
	}

	function closePopup(popupId: string): void {
		visiblePopupIds = new Set([...visiblePopupIds].filter((id) => id !== popupId));
		trackEvent(popupId, 'close');
	}

	async function trackEvent(popupId: string, eventType: string): Promise<void> {
		try {
			await api.post('/api/popups/event', {
				popup_id: popupId,
				event_type: eventType,
				page_url: currentPath,
				session_id: getSessionId()
			});
		} catch {
			// Silently fail — tracking should not break UX
		}
	}

	function getSessionId(): string {
		if (!browser) return '';
		let sid = sessionStorage.getItem('swings_session_id');
		if (!sid) {
			sid = crypto.randomUUID();
			sessionStorage.setItem('swings_session_id', sid);
		}
		return sid;
	}

	async function handleSubmit(popup: Popup, formData: Record<string, unknown>): Promise<void> {
		try {
			await api.post('/api/popups/submit', {
				popup_id: popup.id,
				form_data: formData,
				page_url: currentPath,
				session_id: getSessionId()
			});
		} catch {
			// Silently fail
		}

		if (popup.redirect_url) {
			// SECURITY: an attacker with popup-author access (or a hijacked
			// `popups` row) could set `redirect_url` to an off-domain phishing
			// target. Gate on:
			//   1. Well-formed URL (rejects `javascript:` and malformed values).
			//   2. Same-origin OR relative — rooted in the running page's
			//      origin so preview / staging deploys still work.
			// Downgrade to a log rather than throwing so a bad redirect
			// doesn't break the form submission flow.
			const safe = toSafeRedirect(popup.redirect_url);
			if (safe) {
				window.location.href = safe;
			} else {
				console.warn('[PopupEngine] dropped off-origin redirect_url', popup.redirect_url);
			}
		}
	}

	/**
	 * Return a safe same-origin redirect target, or `null` if the URL is
	 * off-origin / malformed / uses a forbidden scheme. Accepts:
	 *   * absolute URLs whose origin matches `window.location.origin`
	 *   * relative paths (`/foo`, `./bar`, `baz`) which resolve to the
	 *     current origin by the URL constructor
	 */
	function toSafeRedirect(raw: string): string | null {
		if (!browser) return null;
		try {
			const parsed = new URL(raw, window.location.origin);
			if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
				return null;
			}
			if (parsed.origin !== window.location.origin) {
				return null;
			}
			return parsed.toString();
		} catch {
			return null;
		}
	}

	function setupTrigger(popup: Popup): (() => void) | null {
		if (hasBeenShown(popup.id, popup.display_frequency)) return null;

		const config = popup.trigger_config || {};

		switch (popup.trigger_type) {
			case 'on_load': {
				showPopup(popup);
				return null;
			}

			case 'time_delay': {
				const delay = Number(config.delay_ms) || 3000;
				const timer = setTimeout(() => showPopup(popup), delay);
				return () => clearTimeout(timer);
			}

			case 'scroll_percentage': {
				const target = Number(config.percentage) || 50;
				function handleScroll() {
					const scrollHeight = document.documentElement.scrollHeight - window.innerHeight;
					if (scrollHeight <= 0) return;
					const percent = (window.scrollY / scrollHeight) * 100;
					if (percent >= target) {
						showPopup(popup);
						window.removeEventListener('scroll', handleScroll);
					}
				}
				window.addEventListener('scroll', handleScroll, { passive: true });
				return () => window.removeEventListener('scroll', handleScroll);
			}

			case 'exit_intent': {
				if (getDevice() !== 'desktop') return null;
				function handleMouseLeave(e: MouseEvent) {
					if (e.clientY <= 0) {
						showPopup(popup);
						document.removeEventListener('mouseleave', handleMouseLeave);
					}
				}
				document.addEventListener('mouseleave', handleMouseLeave);
				return () => document.removeEventListener('mouseleave', handleMouseLeave);
			}

			case 'click': {
				const selector = String(config.selector || '');
				if (!selector) return null;
				function handleClick(e: Event) {
					const target = e.target as HTMLElement;
					if (target.matches(selector) || target.closest(selector)) {
						showPopup(popup);
					}
				}
				document.addEventListener('click', handleClick);
				return () => document.removeEventListener('click', handleClick);
			}

			case 'inactivity': {
				const timeout = Number(config.timeout_ms) || 30000;
				let timer = setTimeout(() => showPopup(popup), timeout);

				function resetTimer() {
					clearTimeout(timer);
					timer = setTimeout(() => showPopup(popup), timeout);
				}

				const events = ['mousemove', 'keydown', 'scroll', 'touchstart', 'click'] as const;
				events.forEach((evt) =>
					window.addEventListener(evt, resetTimer, { passive: true })
				);

				return () => {
					clearTimeout(timer);
					events.forEach((evt) => window.removeEventListener(evt, resetTimer));
				};
			}

			default:
				return null;
		}
	}

	function cleanupAllTriggers() {
		cleanupFns.forEach((fn) => fn());
		cleanupFns = [];
	}

	async function fetchAndSetupPopups() {
		if (!browser) return;
		if (Date.now() < popupFetchPauseUntil) return;
		const reqId = ++popupRequestSeq;

		cleanupAllTriggers();
		visiblePopupIds = new Set();

		const device = getDevice();
		const userStatus = getUserStatus();

		try {
			const popups = await api.get<Popup[]>(
				`/api/popups/active?page=${encodeURIComponent(currentPath)}&device=${device}&user_status=${userStatus}`,
				{ skipAuth: true }
			);
			if (reqId !== popupRequestSeq) return;
			popupFetchFailures = 0;
			popupFetchPauseUntil = 0;
			activePopups = popups;

			for (const popup of activePopups) {
				const cleanup = setupTrigger(popup);
				if (cleanup) {
					cleanupFns.push(cleanup);
				}
			}
		} catch (err) {
			if (reqId !== popupRequestSeq) return;
			activePopups = [];
			popupFetchFailures += 1;
			const backoffMs = Math.min(60_000, 1_500 * 2 ** (popupFetchFailures - 1));

			// Hard-disable for longer if backend route is missing (common during misdeploys).
			if (err instanceof ApiError && err.status === 404) {
				popupFetchPauseUntil = Date.now() + 10 * 60_000;
				return;
			}

			popupFetchPauseUntil = Date.now() + backoffMs;
		}
	}

	onMount(() => {
		// `afterNavigate` also fires on first load and stays registered until this component unmounts.
		afterNavigate(() => {
			fetchAndSetupPopups();
		});
		return () => {
			cleanupAllTriggers();
		};
	});
</script>

{#each activePopups.filter((p) => visiblePopupIds.has(p.id)) as popup (popup.id)}
	<PopupRenderer
		{popup}
		onclose={() => closePopup(popup.id)}
		onsubmit={(formData) => handleSubmit(popup, formData)}
	/>
{/each}
