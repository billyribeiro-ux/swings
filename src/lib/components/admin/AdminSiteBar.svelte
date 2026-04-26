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
		min-height: 2rem;
		padding: 0.35rem 0.75rem;
		background: linear-gradient(90deg, rgba(15, 23, 42, 0.92) 0%, rgba(30, 41, 59, 0.92) 100%);
		backdrop-filter: blur(10px);
		-webkit-backdrop-filter: blur(10px);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		font-size: var(--fs-2xs);
		font-weight: var(--w-medium);
	}

	.admin-site-bar__link {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.2rem 0.5rem;
		margin: -0.2rem -0.5rem;
		border-radius: var(--radius-md);
		color: var(--color-teal-light);
		font-weight: var(--w-semibold);
		text-decoration: none;
		white-space: nowrap;
		transition:
			color 150ms var(--ease-out),
			background-color 150ms var(--ease-out);
	}

	.admin-site-bar__link:hover {
		color: var(--color-white);
		background-color: rgba(15, 164, 175, 0.12);
	}

	.admin-site-bar__link:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin-site-bar__hint {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-grey-300);
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.admin-site-bar__hint :global(svg) {
		color: var(--color-teal-light);
		flex-shrink: 0;
	}

	@media (min-width: 480px) {
		.admin-site-bar {
			padding: 0.4rem 1rem;
			gap: 1rem;
		}
	}
</style>
