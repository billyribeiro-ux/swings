<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { page } from '$app/state';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import GaugeIcon from 'phosphor-svelte/lib/GaugeIcon';

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
		<span class="admin-site-bar__hint">
			<ShieldCheckIcon size={14} weight="duotone" />
			<span>Signed in as admin</span>
		</span>
		<a href="/admin" class="admin-site-bar__link">
			<GaugeIcon size={14} weight="duotone" />
			<span>Open dashboard</span>
		</a>
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
		gap: 0.5rem;
		min-height: 2.25rem;
		padding: 0.35rem 0.75rem;
		background: linear-gradient(90deg, rgba(15, 23, 42, 0.95) 0%, rgba(30, 41, 59, 0.95) 100%);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		font-size: var(--fs-2xs);
	}

	.admin-site-bar__link {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-teal-light);
		font-weight: var(--w-semibold);
		text-decoration: none;
		white-space: nowrap;
		transition: color 150ms var(--ease-out);
	}

	.admin-site-bar__link:hover {
		color: var(--color-white);
	}

	.admin-site-bar__hint {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-grey-400);
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	@media (min-width: 480px) {
		.admin-site-bar {
			padding: 0.4rem 1rem;
			gap: 1rem;
		}
	}
</style>
