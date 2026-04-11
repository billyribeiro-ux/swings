<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { page } from '$app/state';
	import Gauge from 'phosphor-svelte/lib/Gauge';

	const visible = $derived(
		auth.isAuthenticated &&
			auth.isAdmin &&
			!page.url.pathname.startsWith('/admin') &&
			!page.url.pathname.startsWith('/dashboard') &&
			!page.url.pathname.startsWith('/login') &&
			!page.url.pathname.startsWith('/register')
	);
</script>

{#if visible}
	<div class="admin-site-bar">
		<a href="/admin" class="admin-site-bar__link">
			<Gauge size={18} weight="duotone" />
			<span>Dashboard</span>
		</a>
		<span class="admin-site-bar__hint">You are signed in as admin</span>
	</div>
{/if}

<style>
	.admin-site-bar {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: calc(var(--z-50, 50) + 20);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
		min-height: 2.5rem;
		padding: 0.35rem 1rem;
		background: linear-gradient(90deg, #0f172a 0%, #1e293b 100%);
		border-bottom: 1px solid rgba(15, 164, 175, 0.35);
		font-size: var(--fs-xs);
	}

	.admin-site-bar__link {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-teal-light);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	.admin-site-bar__link:hover {
		text-decoration: underline;
	}

	.admin-site-bar__hint {
		color: var(--color-grey-500);
	}
</style>
