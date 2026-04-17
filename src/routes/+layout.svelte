<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import Nav from '$lib/components/ui/Nav.svelte';
	import Footer from '$lib/components/ui/Footer.svelte';
	import FloatingButton from '$lib/components/ui/FloatingButton.svelte';
	import TradersModal from '$lib/components/traders/TradersModal.svelte';
	import AnalyticsBeacon from '$lib/analytics/AnalyticsBeacon.svelte';
	import AdminSiteBar from '$lib/components/admin/AdminSiteBar.svelte';
	import PopupEngine from '$lib/components/popups/PopupEngine.svelte';
	import ConsentBanner from '$lib/components/consent/ConsentBanner.svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { organizationSchema, webSiteSchema, buildJsonLd } from '$lib/seo/jsonld';
	import { SITE } from '$lib/seo/config';
	import { STUB_BANNER_CONFIG, fetchBannerConfig, type BannerConfig } from '$lib/api/consent';
	import { setupGateScanner } from '$lib/consent/gate';
	import { installGcmBridge } from '$lib/consent/gcm';
	import { installTcfApi } from '$lib/consent/tcf';
	import { consent } from '$lib/stores/consent.svelte';
	import { setLocale } from '$lib/i18n/paraglide';

	let { children } = $props();

	// CONSENT-01: render the stub synchronously so the banner has no FOUC, then
	// swap in the live config once the network round-trip resolves. Failures
	// stay on the stub — `fetchBannerConfig` handles that internally.
	let bannerConfig = $state<BannerConfig>(STUB_BANNER_CONFIG);
	$effect(() => {
		if (!browser) return;
		// CONSENT-06: pick up `navigator.language` before the fetch so the
		// i18n layer renders in the subject's preferred locale before the
		// network response lands. The server-side banner response then
		// overrides both the copy and (if the admin configured a locale
		// variant) the locale itself.
		if (typeof navigator !== 'undefined' && navigator.language) {
			setLocale(navigator.language);
		}
		void fetchBannerConfig().then((cfg) => {
			bannerConfig = cfg;
			if (cfg.locale) setLocale(cfg.locale);
		});
	});

	// CONSENT-02: install the script-hydration scanner. Runs once on mount and
	// re-scans on every consent update. Scripts gated via `data-consent-category`
	// (in `app.html` or injected by MDX content) activate as soon as the
	// matching category is granted.
	//
	// CONSENT-04: install GCM v2 + TCF v2.2 bridges in the same browser-only
	// effect. Both are idempotent so a hot-reload cycle doesn't double-push
	// frames. Order matters: GCM defaults must be written before any tag
	// script fires, so this installer runs before the script-gate scanner.
	$effect(() => {
		if (!browser) return;
		installGcmBridge(consent.state.categories);
		installTcfApi(consent.state.categories);
		setupGateScanner();
	});

	const appRoutes = ['/dashboard', '/admin', '/login', '/register'];
	const isAppRoute = $derived(appRoutes.some((r) => page.url.pathname.startsWith(r)));
	const noindexRoutes = ['/success'];
	const isNoindexRoute = $derived(
		isAppRoute || noindexRoutes.some((r) => page.url.pathname.startsWith(r))
	);

	/** Offset nav when WordPress-style admin bar is visible */
	const wpAdminOffset = $derived(
		!isAppRoute &&
			auth.isAuthenticated &&
			auth.isAdmin &&
			!['/dashboard', '/login', '/register'].some((p) => page.url.pathname.startsWith(p))
	);

	const _globalJsonLd = buildJsonLd([organizationSchema(), webSiteSchema()]);
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
	<meta name="author" content="Billy Ribeiro" />
	<meta name="publisher" content={SITE.name} />
	{#if isNoindexRoute}
		<meta name="robots" content="noindex, nofollow" />
	{/if}
	<script type="application/ld+json">{_globalJsonLd}</script>
</svelte:head>

<AnalyticsBeacon />

{#if isAppRoute}
	{@render children()}
{:else}
	<div
		class="public-shell"
		class:public-shell--wp-admin={wpAdminOffset}
		data-sveltekit-preload-data="hover"
	>
		<AdminSiteBar />
		<Nav />

		<main>
			{@render children()}
		</main>

		<Footer />
	</div>
	<FloatingButton />
	<TradersModal />
{/if}

{#if browser}
	<ConsentBanner config={bannerConfig} />
	<PopupEngine />
{/if}

<style>
	:global(.public-shell--wp-admin .nav) {
		top: 2.5rem;
	}
</style>
