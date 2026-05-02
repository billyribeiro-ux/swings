<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth.svelte';
	import PackageIcon from 'phosphor-svelte/lib/PackageIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import TagIcon from 'phosphor-svelte/lib/TagIcon';
	import MapPinIcon from 'phosphor-svelte/lib/MapPinIcon';
	import CreditCardIcon from 'phosphor-svelte/lib/CreditCardIcon';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';
	import SignOutIcon from 'phosphor-svelte/lib/SignOutIcon';
	import type { ResolvedPathname } from '$app/types';

	type AccountNavItem = {
		href: ResolvedPathname;
		label: string;
		icon: typeof PackageIcon;
	};

	let { children } = $props();

	const navItems: AccountNavItem[] = [
		{ href: resolve('/dashboard/account/orders'), label: 'My Orders', icon: PackageIcon },
		{
			href: resolve('/dashboard/account/subscriptions'),
			label: 'My Subscriptions',
			icon: LightningIcon
		},
		{ href: resolve('/dashboard/account/coupons'), label: 'Coupons', icon: TagIcon },
		{
			href: resolve('/dashboard/account/billing-address'),
			label: 'Billing Address',
			icon: MapPinIcon
		},
		{
			href: resolve('/dashboard/account/payment-methods'),
			label: 'Payment Methods',
			icon: CreditCardIcon
		},
		{
			href: resolve('/dashboard/account/details'),
			label: 'Account Details',
			icon: UserCircleIcon
		}
	];

	const currentPath = $derived(page.url.pathname);

	function isActive(href: string): boolean {
		return currentPath === href || currentPath.startsWith(href + '/');
	}

	async function handleLogout() {
		await auth.logout();
		await goto(resolve('/login'));
	}
</script>

<div class="acct">
	<aside class="acct__sidebar" aria-label="Account navigation">
		<nav class="acct__nav">
			{#each navItems as item (item.href)}
				{@const active = isActive(item.href)}
				<a
					href={item.href}
					class="acct__nav-link"
					class:acct__nav-link--active={active}
					aria-current={active ? 'page' : undefined}
				>
					<item.icon size={20} weight="duotone" />
					<span class="acct__nav-label">{item.label}</span>
				</a>
			{/each}
		</nav>

		<div class="acct__divider" role="presentation"></div>

		<button type="button" class="acct__logout" onclick={handleLogout}>
			<SignOutIcon size={20} weight="duotone" />
			<span class="acct__nav-label">Log out</span>
		</button>
	</aside>

	<section class="acct__content">
		{@render children()}
	</section>
</div>

<style>
	.acct {
		display: grid;
		grid-template-columns: 240px 1fr;
		gap: 2rem;
		align-items: start;
	}

	.acct__sidebar {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 0.75rem;
		position: sticky;
		top: 2rem;
	}

	.acct__nav {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.acct__nav-link {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.7rem 0.85rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		border-left: 3px solid transparent;
		transition:
			color 200ms var(--ease-out),
			background-color 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}

	.acct__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.04);
	}

	.acct__nav-link--active {
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.1);
		border-left-color: var(--color-teal);
		font-weight: var(--w-semibold);
	}

	.acct__nav-label {
		white-space: nowrap;
	}

	.acct__divider {
		height: 1px;
		background-color: rgba(255, 255, 255, 0.08);
		margin: 0.6rem 0.25rem;
	}

	.acct__logout {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.7rem 0.85rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		background: transparent;
		border: none;
		border-left: 3px solid transparent;
		cursor: pointer;
		text-align: left;
		transition:
			color 200ms var(--ease-out),
			background-color 200ms var(--ease-out);
	}

	.acct__logout:hover {
		color: var(--color-red);
		background-color: rgba(239, 68, 68, 0.08);
	}

	.acct__content {
		min-width: 0;
	}

	@media (max-width: 768px) {
		.acct {
			grid-template-columns: 1fr;
			gap: 1rem;
		}

		.acct__sidebar {
			position: static;
			padding: 0.5rem;
			gap: 0;
		}

		.acct__nav {
			flex-direction: row;
			flex-wrap: nowrap;
			overflow-x: auto;
			gap: 0.4rem;
			scrollbar-width: thin;
		}

		.acct__nav-link {
			flex: 0 0 auto;
			padding: 0.5rem 0.85rem;
			border-left: none;
			border-bottom: 2px solid transparent;
			border-radius: var(--radius-full);
			background-color: rgba(255, 255, 255, 0.03);
		}

		.acct__nav-link--active {
			border-left: none;
			border-bottom-color: var(--color-teal);
			background-color: rgba(15, 164, 175, 0.12);
		}

		.acct__divider {
			display: none;
		}

		.acct__logout {
			margin-top: 0.5rem;
			justify-content: center;
			border-left: none;
			background-color: rgba(255, 255, 255, 0.03);
		}
	}
</style>
