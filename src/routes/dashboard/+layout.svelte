<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import House from 'phosphor-svelte/lib/House';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import UserCircle from 'phosphor-svelte/lib/UserCircle';
	import SignOut from 'phosphor-svelte/lib/SignOut';
	import { SITE } from '$lib/seo/config';

	let { children } = $props();

	onMount(() => {
		if (!auth.isAuthenticated) {
			goto('/login');
		}
	});

	function handleLogout() {
		auth.logout();
		goto('/login');
	}

	const navItems = [
		{ href: '/dashboard', label: 'Overview', icon: House },
		{ href: '/dashboard/watchlists', label: 'Watchlists', icon: ListChecks },
		{ href: '/dashboard/courses', label: 'Courses', icon: BookOpen },
		{ href: '/dashboard/account', label: 'Account', icon: UserCircle }
	];
</script>

{#if auth.isAuthenticated}
	<div class="dash">
		<aside class="dash__sidebar">
			<a href="/" class="dash__logo">
				<span class="dash__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="dash__logo-accent">{SITE.logoBrandAccent}</span>
			</a>

			<nav class="dash__nav">
				{#each navItems as item (item.href)}
					<a href={item.href} class="dash__nav-link">
						<item.icon size={20} weight="duotone" />
						<span>{item.label}</span>
					</a>
				{/each}
			</nav>

			<div class="dash__sidebar-footer">
				{#if auth.isAdmin}
					<a href="/admin" class="dash__nav-link dash__nav-link--admin"> Admin Panel </a>
				{/if}
				<button onclick={handleLogout} class="dash__logout">
					<SignOut size={20} weight="duotone" />
					<span>Sign Out</span>
				</button>
			</div>
		</aside>

		<div class="dash__main">
			<header class="dash__header">
				<div>
					<h2 class="dash__greeting">Welcome back, {auth.user?.name?.split(' ')[0]}</h2>
				</div>
			</header>

			<div class="dash__content">
				{@render children()}
			</div>
		</div>
	</div>
{/if}

<style>
	.dash {
		display: flex;
		min-height: 100vh;
		background-color: var(--color-navy-deep);
	}

	.dash__sidebar {
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

	.dash__logo {
		display: flex;
		gap: 0.3rem;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 2rem;
		padding: 0 0.5rem;
	}

	.dash__logo-brand {
		color: var(--color-white);
	}

	.dash__logo-accent {
		color: var(--color-teal);
	}

	.dash__nav {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
	}

	.dash__nav-link {
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

	.dash__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.dash__nav-link--admin {
		color: var(--color-teal);
		border: 1px solid rgba(15, 164, 175, 0.2);
		justify-content: center;
		margin-bottom: 0.5rem;
	}

	.dash__sidebar-footer {
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 1rem;
	}

	.dash__logout {
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

	.dash__logout:hover {
		color: #fca5a5;
		background-color: rgba(239, 68, 68, 0.08);
	}

	.dash__main {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow-y: auto;
	}

	.dash__header {
		padding: 1.5rem 2rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.dash__greeting {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.dash__content {
		flex: 1;
		padding: 2rem;
	}

	@media (max-width: 768px) {
		.dash {
			flex-direction: column;
		}

		.dash__sidebar {
			width: 100%;
			height: auto;
			position: relative;
			flex-direction: row;
			flex-wrap: wrap;
			align-items: center;
			padding: 1rem;
		}

		.dash__nav {
			flex-direction: row;
			gap: 0.25rem;
			flex: initial;
		}

		.dash__sidebar-footer {
			border-top: none;
			padding-top: 0;
			margin-left: auto;
		}

		.dash__content {
			padding: 1rem;
		}
	}
</style>
