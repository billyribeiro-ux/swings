<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import HouseIcon from 'phosphor-svelte/lib/HouseIcon';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';
	import SignOutIcon from 'phosphor-svelte/lib/SignOutIcon';
	import { SITE } from '$lib/seo/config';

	let { children } = $props();

	onMount(() => {
		if (!auth.isAuthenticated) {
			goto(resolve('/login'));
		}
	});

	function handleLogout() {
		auth.logout();
		goto(resolve('/login'));
	}

	const navItems = [
		{ href: resolve('/dashboard'), label: 'Overview', icon: HouseIcon, exact: true },
		{
			href: resolve('/dashboard/watchlists'),
			label: 'Watchlists',
			icon: ListChecksIcon,
			exact: false
		},
		{ href: resolve('/dashboard/courses'), label: 'Courses', icon: BookOpenIcon, exact: false },
		{ href: resolve('/dashboard/account'), label: 'Account', icon: UserCircleIcon, exact: false }
	];

	const currentPath = $derived(page.url.pathname);

	function isActive(href: string, exact: boolean): boolean {
		if (exact) return currentPath === href;
		return currentPath === href || currentPath.startsWith(href + '/');
	}

	function initials(name: string | undefined): string {
		if (!name) return '?';
		const parts = name.trim().split(/\s+/);
		if (parts.length === 1) return parts[0]![0]!.toUpperCase();
		return (parts[0]![0]! + parts[parts.length - 1]![0]!).toUpperCase();
	}
</script>

{#if auth.isAuthenticated}
	<div class="dash">
		<aside class="dash__sidebar">
			<a href={resolve('/')} class="dash__logo">
				<span class="dash__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="dash__logo-accent">{SITE.logoBrandAccent}</span>
			</a>

			<nav class="dash__nav">
				{#each navItems as item (item.href)}
					{@const active = isActive(item.href, item.exact)}
					<a
						href={item.href}
						class="dash__nav-link"
						class:dash__nav-link--active={active}
						aria-current={active ? 'page' : undefined}
					>
						<item.icon size={20} weight="duotone" />
						<span>{item.label}</span>
					</a>
				{/each}
			</nav>

			<div class="dash__sidebar-footer">
				{#if auth.isAdmin}
					<a href={resolve('/admin')} class="dash__nav-link dash__nav-link--admin">
						Admin Panel
					</a>
				{/if}

				<div class="dash__user">
					<div class="dash__avatar">
						{#if auth.user?.avatar_url}
							<img
								src={auth.user.avatar_url}
								alt={auth.user.name ?? 'Member avatar'}
								class="dash__avatar-img"
							/>
						{:else}
							<span class="dash__avatar-initials" aria-hidden="true">
								{initials(auth.user?.name)}
							</span>
						{/if}
					</div>
					<div class="dash__user-meta">
						<span class="dash__user-name">{auth.user?.name ?? 'Member'}</span>
						<span class="dash__user-role">Member</span>
					</div>
				</div>

				<button onclick={handleLogout} class="dash__logout">
					<SignOutIcon size={20} weight="duotone" />
					<span>Sign Out</span>
				</button>
			</div>
		</aside>

		<div class="dash__main">
			<div class="dash__content">
				{@render children()}
			</div>
		</div>

		<nav class="dash__tabbar" aria-label="Primary">
			{#each navItems as item (item.href)}
				{@const active = isActive(item.href, item.exact)}
				{@const isAccount = item.label === 'Account'}
				<a
					href={item.href}
					class="dash__tab"
					class:dash__tab--active={active}
					aria-current={active ? 'page' : undefined}
				>
					{#if isAccount && auth.user?.avatar_url}
						<img
							src={auth.user.avatar_url}
							alt=""
							class="dash__tab-avatar"
							aria-hidden="true"
						/>
					{:else}
						<item.icon size={20} weight="duotone" />
					{/if}
					<span class="dash__tab-label">{item.label}</span>
				</a>
			{/each}
		</nav>
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
		border-left: 3px solid transparent;
	}

	.dash__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.dash__nav-link--active {
		color: var(--color-white);
		background-color: rgba(15, 164, 175, 0.08);
		border-left: 3px solid var(--color-teal);
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
		display: flex;
		flex-direction: column;
	}

	.dash__user {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		padding: 0.65rem 0.5rem;
		border-radius: var(--radius-lg);
		margin-bottom: 0.75rem;
		min-width: 0;
	}

	.dash__avatar {
		width: 36px;
		height: 36px;
		border-radius: var(--radius-full);
		overflow: hidden;
		flex-shrink: 0;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
	}

	.dash__avatar-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.dash__avatar-initials {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: var(--color-white);
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		letter-spacing: 0.02em;
	}

	.dash__user-meta {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		min-width: 0;
		flex: 1;
	}

	.dash__user-name {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.dash__user-role {
		display: inline-flex;
		align-self: flex-start;
		align-items: center;
		font-size: var(--fs-2xs);
		font-weight: var(--w-semibold);
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.15);
		border-radius: var(--radius-full);
		padding: 0.15rem 0.5rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
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

	.dash__content {
		flex: 1;
		padding: 2rem;
	}

	.dash__tabbar {
		display: none;
	}

	@media (max-width: 768px) {
		.dash {
			flex-direction: column;
		}

		.dash__sidebar {
			display: none;
		}

		.dash__content {
			padding: 1rem;
			padding-bottom: 4rem;
		}

		.dash__tabbar {
			display: grid;
			grid-template-columns: repeat(4, 1fr);
			position: fixed;
			bottom: 0;
			left: 0;
			right: 0;
			height: 56px;
			background-color: var(--color-navy);
			border-top: 1px solid rgba(255, 255, 255, 0.06);
			z-index: 50;
		}

		.dash__tab {
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			gap: 0.15rem;
			color: var(--color-grey-400);
			text-decoration: none;
			font-size: var(--fs-xs);
			font-weight: var(--w-medium);
			transition: color 200ms var(--ease-out);
		}

		.dash__tab--active {
			color: var(--color-teal);
		}

		.dash__tab-label {
			font-size: 0.7rem;
			line-height: 1;
		}

		.dash__tab-avatar {
			width: 22px;
			height: 22px;
			border-radius: var(--radius-full);
			object-fit: cover;
			border: 1.5px solid rgba(15, 164, 175, 0.4);
		}
	}
</style>
