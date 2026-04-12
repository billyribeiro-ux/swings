<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import { api } from '$lib/api/client';
	import { auth } from '$lib/stores/auth.svelte';
	import type { Popup } from '$lib/api/types';
	import PopupRenderer from './PopupRenderer.svelte';

	const STORAGE_KEY_PREFIX = 'swings_popup_';
	const SESSION_KEY_PREFIX = 'swings_popup_session_';

	let activePopups = $state<Popup[]>([]);
	let visiblePopupIds = $state<Set<string>>(new Set());
	let cleanupFns: Array<() => void> = [];

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
			window.location.href = popup.redirect_url;
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
				events.forEach((evt) => window.addEventListener(evt, resetTimer, { passive: true }));

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

		cleanupAllTriggers();
		visiblePopupIds = new Set();

		const device = getDevice();
		const userStatus = getUserStatus();

		try {
			const popups = await api.get<Popup[]>(
				`/api/popups/active?page=${encodeURIComponent(currentPath)}&device=${device}&user_status=${userStatus}`,
				{ skipAuth: true }
			);
			activePopups = popups;

			for (const popup of activePopups) {
				const cleanup = setupTrigger(popup);
				if (cleanup) {
					cleanupFns.push(cleanup);
				}
			}
		} catch {
			activePopups = [];
		}
	}

	onMount(() => {
		fetchAndSetupPopups();
		return () => cleanupAllTriggers();
	});

	// Re-fetch when route changes
	let previousPath = $state('');
	$effect(() => {
		if (currentPath !== previousPath) {
			previousPath = currentPath;
			fetchAndSetupPopups();
		}
	});
</script>

{#each activePopups.filter((p) => visiblePopupIds.has(p.id)) as popup (popup.id)}
	<PopupRenderer
		{popup}
		onclose={() => closePopup(popup.id)}
		onsubmit={(formData) => handleSubmit(popup, formData)}
	/>
{/each}
