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
	import { auth } from '$lib/stores/auth.svelte';
	import { organizationSchema, webSiteSchema, buildJsonLd } from '$lib/seo/jsonld';

	let { children } = $props();

	const appRoutes = ['/dashboard', '/admin', '/login', '/register'];
	const isAppRoute = $derived(appRoutes.some((r) => page.url.pathname.startsWith(r)));

	/** Offset nav when WordPress-style admin bar is visible */
	const wpAdminOffset = $derived(
		!isAppRoute &&
			auth.isAuthenticated &&
			auth.isAdmin &&
			!['/dashboard', '/login', '/register'].some((p) => page.url.pathname.startsWith(p))
	);

	const globalJsonLd = buildJsonLd([organizationSchema(), webSiteSchema()]);
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
	<meta name="author" content="Billy Ribeiro" />
	<meta name="publisher" content="Explosive Swings" />
	{@html `<script type="application/ld+json">${globalJsonLd}</script>`}
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
	<PopupEngine />
{/if}

<style>
	:global(.public-shell--wp-admin .nav) {
		top: 2.5rem;
	}
</style>
