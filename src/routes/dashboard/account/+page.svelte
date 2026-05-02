<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	onMount(() => {
		void goto(resolve('/dashboard/account/subscriptions'), { replaceState: true });
	});
</script>

<svelte:head><title>Account - Precision Options Signals</title></svelte:head>

<p class="loading">Loading…</p>

<style>
	.loading {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		padding: 1rem 0;
	}
</style>
