<script lang="ts">
	import '../app.css';
	import Nav from '$lib/components/ui/Nav.svelte';
	import Footer from '$lib/components/ui/Footer.svelte';
	import FloatingButton from '$lib/components/ui/FloatingButton.svelte';
	import TradersModal from '$lib/components/traders/TradersModal.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
</svelte:head>

<div data-sveltekit-preload-data="hover">
	<Nav />

	<main>
		{@render children()}
	</main>

	<Footer />
</div>
<FloatingButton />
<TradersModal />
