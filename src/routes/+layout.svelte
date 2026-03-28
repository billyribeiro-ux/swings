<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import Nav from '$lib/components/ui/Nav.svelte';
	import Footer from '$lib/components/ui/Footer.svelte';
	import FloatingButton from '$lib/components/ui/FloatingButton.svelte';
	import TradersModal from '$lib/components/traders/TradersModal.svelte';

	let { children } = $props();

	const appRoutes = ['/dashboard', '/admin', '/login', '/register'];
	const isAppRoute = $derived(appRoutes.some((r) => $page.url.pathname.startsWith(r)));
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
</svelte:head>

{#if isAppRoute}
	{@render children()}
{:else}
	<div data-sveltekit-preload-data="hover">
		<Nav />

		<main>
			{@render children()}
		</main>

		<Footer />
	</div>
	<FloatingButton />
	<TradersModal />
{/if}
