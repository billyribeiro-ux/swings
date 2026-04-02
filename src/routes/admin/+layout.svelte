<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import ChartBar from 'phosphor-svelte/lib/ChartBar';
	import Users from 'phosphor-svelte/lib/Users';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import Article from 'phosphor-svelte/lib/Article';
	import SignOut from 'phosphor-svelte/lib/SignOut';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';

	let { children } = $props();

	onMount(() => {
		if (!auth.isAuthenticated || !auth.isAdmin) {
			goto('/login');
		}
	});

	function handleLogout() {
		auth.logout();
		goto('/login');
	}

	const navItems = [
		{ href: '/admin', label: 'Dashboard', icon: ChartBar },
		{ href: '/admin/members', label: 'Members', icon: Users },
		{ href: '/admin/watchlists', label: 'Watchlists', icon: ListChecks }
	];

	const blogItems = [
		{ href: '/admin/blog', label: 'All Posts' },
		{ href: '/admin/blog/new', label: 'New Post' },
		{ href: '/admin/blog/categories', label: 'Categories' },
		{ href: '/admin/blog/tags', label: 'Tags' },
		{ href: '/admin/blog/media', label: 'Media' }
	];
</script>

{#if auth.isAdmin}
	<div class="admin">
		<aside class="admin__sidebar">
			<div class="admin__sidebar-top">
				<a href="/" class="admin__logo">
					<span class="admin__logo-brand">Explosive</span>
					<span class="admin__logo-accent">Swings</span>
				</a>
				<span class="admin__badge">Admin</span>
			</div>

			<nav class="admin__nav">
				{#each navItems as item (item.href)}
					<a href={item.href} class="admin__nav-link">
						<item.icon size={20} weight="duotone" />
						<span>{item.label}</span>
					</a>
				{/each}

				<div class="admin__nav-section">
					<div class="admin__nav-link admin__nav-link--header">
						<Article size={20} weight="duotone" />
						<span>Blog</span>
					</div>
					{#each blogItems as item (item.href)}
						<a href={item.href} class="admin__nav-sublink">
							{item.label}
						</a>
					{/each}
				</div>
			</nav>

			<div class="admin__sidebar-footer">
				<a href="/dashboard" class="admin__nav-link admin__nav-link--back">
					<ArrowLeft size={18} />
					<span>Member Dashboard</span>
				</a>
				<button onclick={handleLogout} class="admin__logout">
					<SignOut size={20} weight="duotone" />
					<span>Sign Out</span>
				</button>
			</div>
		</aside>

		<div class="admin__main">
			<div class="admin__content">
				{@render children()}
			</div>
		</div>
	</div>
{/if}

<style>
	.admin {
		display: flex;
		min-height: 100vh;
		background-color: var(--color-navy-deep);
	}

	.admin__sidebar {
		width: 16rem;
		background-color: var(--color-navy);
		border-right: 1px solid rgba(255, 255, 255, 0.06);
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		position: sticky;
		top: 0;
		height: 100vh;
	}

	.admin__sidebar-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 2rem;
		padding: 0 0.5rem;
	}

	.admin__logo {
		display: flex;
		gap: 0.3rem;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
	}

	.admin__logo-brand {
		color: var(--color-white);
	}

	.admin__logo-accent {
		color: var(--color-teal);
	}

	.admin__badge {
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: #f59e0b;
		background-color: rgba(245, 158, 11, 0.12);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.admin__nav {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
	}

	.admin__nav-link {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.65rem 0.75rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition: all 200ms var(--ease-out);
	}

	.admin__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.admin__nav-link--back {
		color: var(--color-teal);
		margin-bottom: 0.5rem;
	}

	.admin__nav-link--header {
		cursor: default;
		margin-top: 0.5rem;
		color: var(--color-grey-300);
		font-weight: var(--w-semibold);
	}

	.admin__nav-section {
		display: flex;
		flex-direction: column;
	}

	.admin__nav-sublink {
		display: block;
		padding: 0.4rem 0.75rem 0.4rem 2.75rem;
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		text-decoration: none;
		border-radius: var(--radius-md);
		transition: all 200ms var(--ease-out);
	}

	.admin__nav-sublink:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.admin__sidebar-footer {
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 1rem;
	}

	.admin__logout {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.65rem 0.75rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		background: none;
		border: none;
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}

	.admin__logout:hover {
		color: #fca5a5;
		background-color: rgba(239, 68, 68, 0.08);
	}

	.admin__main {
		flex: 1;
		overflow-y: auto;
	}

	.admin__content {
		padding: 2rem;
	}

	@media (max-width: 768px) {
		.admin {
			flex-direction: column;
		}

		.admin__sidebar {
			width: 100%;
			height: auto;
			position: relative;
			flex-direction: row;
			flex-wrap: wrap;
			align-items: center;
			padding: 1rem;
		}

		.admin__nav {
			flex-direction: row;
			gap: 0.25rem;
			flex: initial;
		}

		.admin__sidebar-footer {
			border-top: none;
			padding-top: 0;
			margin-left: auto;
			display: flex;
			gap: 0.5rem;
		}

		.admin__content {
			padding: 1rem;
		}
	}
</style>
