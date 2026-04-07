<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AuthResponse } from '$lib/api/types';
	import { onMount, onDestroy } from 'svelte';
	import ChartBar from 'phosphor-svelte/lib/ChartBar';
	import Users from 'phosphor-svelte/lib/Users';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import Article from 'phosphor-svelte/lib/Article';
	import UserCircle from 'phosphor-svelte/lib/UserCircle';
	import SignOut from 'phosphor-svelte/lib/SignOut';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import List from 'phosphor-svelte/lib/List';
	import X from 'phosphor-svelte/lib/X';
	import CaretDown from 'phosphor-svelte/lib/CaretDown';
	import CommandPalette from '$lib/components/admin/CommandPalette.svelte';

	let { children } = $props();

	let paletteOpen = $state(false);

	function handleGlobalKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}

	onMount(() => window.addEventListener('keydown', handleGlobalKey));
	onDestroy(() => window.removeEventListener('keydown', handleGlobalKey));

	let mobileMenuOpen = $state(false);
	let blogSubmenuOpen = $state(false);

	const publicRoutes = ['/admin/forgot-password', '/admin/reset-password'];
	const isPublicRoute = $derived(publicRoutes.some((r) => $page.url.pathname.startsWith(r)));

	let email = $state('');
	let password = $state('');
	let loginError = $state('');
	let loginLoading = $state(false);

	async function handleLogin(e: Event) {
		e.preventDefault();
		loginError = '';
		loginLoading = true;

		try {
			const res = await api.post<AuthResponse>(
				'/api/auth/login',
				{ email, password },
				{ skipAuth: true }
			);

			if (res.user.role.toLowerCase() !== 'admin') {
				loginError = 'Access denied. Admin credentials required.';
				loginLoading = false;
				return;
			}

			auth.setAuth(res.user, res.access_token, res.refresh_token);
		} catch (err) {
			if (err instanceof ApiError) {
				loginError = err.status === 401 ? 'Invalid email or password' : err.message;
			} else {
				loginError = 'Something went wrong. Please try again.';
			}
		} finally {
			loginLoading = false;
		}
	}

	function handleLogout() {
		auth.logout();
	}

	const navItems = [
		{ href: '/admin', label: 'Dashboard', icon: ChartBar },
		{ href: '/admin/members', label: 'Members', icon: Users },
		{ href: '/admin/watchlists', label: 'Watchlists', icon: ListChecks },
		{ href: '/admin/author', label: 'Author Profile', icon: UserCircle }
	];

	const blogItems = [
		{ href: '/admin/blog', label: 'All Posts' },
		{ href: '/admin/blog/new', label: 'New Post' },
		{ href: '/admin/blog/categories', label: 'Categories' },
		{ href: '/admin/blog/tags', label: 'Tags' },
		{ href: '/admin/blog/media', label: 'Media' }
	];
</script>

{#if isPublicRoute}
	{@render children()}
{:else if !auth.isAuthenticated || !auth.isAdmin}
	<div class="admin-login">
		<div class="admin-login__card">
			<div class="admin-login__header">
				<a href="/" class="admin-login__logo">
					<span class="admin-login__logo-brand">Explosive</span>
					<span class="admin-login__logo-accent">Swings</span>
				</a>
				<span class="admin-login__badge">Admin</span>
				<h1 class="admin-login__title">Admin Login</h1>
				<p class="admin-login__subtitle">Enter your credentials to access the admin panel</p>
			</div>

			{#if loginError}
				<div class="admin-login__error">{loginError}</div>
			{/if}

			<form onsubmit={handleLogin} class="admin-login__form">
				<div class="admin-login__field">
					<label for="admin-email" class="admin-login__label">Email</label>
					<input
						id="admin-email"
						name="email"
						type="email"
						bind:value={email}
						required
						autocomplete="email"
						class="admin-login__input"
						placeholder="admin@example.com"
					/>
				</div>

				<div class="admin-login__field">
					<label for="admin-password" class="admin-login__label">Password</label>
					<input
						id="admin-password"
						name="password"
						type="password"
						bind:value={password}
						required
						autocomplete="current-password"
						class="admin-login__input"
						placeholder="Enter your password"
					/>
				</div>

				<button type="submit" disabled={loginLoading} class="admin-login__submit">
					{loginLoading ? 'Signing in...' : 'Sign In'}
				</button>
			</form>

			<a href="/admin/forgot-password" class="admin-login__forgot">Forgot password?</a>
			<a href="/" class="admin-login__back">← Back to site</a>
		</div>
	</div>
{:else}
	<div class="admin">
		<!-- Mobile Menu Overlay -->
		{#if mobileMenuOpen}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="admin__overlay"
				role="button"
				tabindex="-1"
				onclick={() => (mobileMenuOpen = false)}
				onkeydown={(e) => e.key === 'Escape' && (mobileMenuOpen = false)}
			></div>
		{/if}

		<!-- Mobile Top Bar -->
		<header class="admin__mobile-header">
			<button class="admin__menu-toggle" onclick={() => (mobileMenuOpen = !mobileMenuOpen)}>
				{#if mobileMenuOpen}
					<X size={24} weight="bold" />
				{:else}
					<List size={24} weight="bold" />
				{/if}
			</button>
			<a href="/" class="admin__mobile-logo">
				<span class="admin__logo-brand">Explosive</span>
				<span class="admin__logo-accent">Swings</span>
			</a>
			<span class="admin__badge admin__badge--mobile">Admin</span>
		</header>

		<aside class="admin__sidebar" class:admin__sidebar--open={mobileMenuOpen}>
			<div class="admin__sidebar-top">
				<a href="/" class="admin__logo">
					<span class="admin__logo-brand">Explosive</span>
					<span class="admin__logo-accent">Swings</span>
				</a>
				<span class="admin__badge">Admin</span>
			</div>

			<nav class="admin__nav">
				{#each navItems as item (item.href)}
					<a
						href={item.href}
						class="admin__nav-link"
						class:admin__nav-link--active={$page.url.pathname === item.href}
						onclick={() => (mobileMenuOpen = false)}
					>
						<item.icon size={20} weight="duotone" />
						<span>{item.label}</span>
					</a>
				{/each}

				<div class="admin__nav-section">
					<button
						class="admin__nav-link admin__nav-link--header"
						onclick={() => (blogSubmenuOpen = !blogSubmenuOpen)}
					>
						<Article size={20} weight="duotone" />
						<span>Blog</span>
						<CaretDown
							size={16}
							class="admin__nav-caret{blogSubmenuOpen ? ' admin__nav-caret--open' : ''}"
						/>
					</button>
					{#if blogSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each blogItems as item (item.href)}
								<a
									href={item.href}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										blogSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>
			</nav>

			<div class="admin__sidebar-footer">
				<a
					href="/dashboard"
					class="admin__nav-link admin__nav-link--back"
					onclick={() => (mobileMenuOpen = false)}
				>
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
		<CommandPalette open={paletteOpen} onClose={() => (paletteOpen = false)} />
	</div>
{/if}

<style>
	/* Mobile-first base styles */
	.admin {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		background-color: var(--color-navy-deep);
	}

	.admin__overlay {
		display: none;
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.6);
		z-index: 40;
	}

	.admin__mobile-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		background-color: var(--color-navy);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		position: sticky;
		top: 0;
		z-index: 30;
	}

	.admin__menu-toggle {
		width: 2.75rem;
		height: 2.75rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}

	.admin__menu-toggle:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}

	.admin__mobile-logo {
		display: flex;
		gap: 0.3rem;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
	}

	.admin__badge--mobile {
		display: inline-block;
	}

	.admin__sidebar {
		position: fixed;
		top: 0;
		left: 0;
		width: 16rem;
		height: 100vh;
		background-color: var(--color-navy);
		border-right: 1px solid rgba(255, 255, 255, 0.06);
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		z-index: 50;
		transform: translateX(-100%);
		transition: transform 300ms var(--ease-out);
	}

	.admin__sidebar--open {
		transform: translateX(0);
	}

	.admin__overlay {
		display: block;
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
		padding: 0.25rem 0.6rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.admin__badge--mobile {
		display: none;
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
		padding: 0.75rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		cursor: pointer;
		background: none;
		border: none;
		width: 100%;
		text-align: left;
		transition: all 200ms var(--ease-out);
	}

	.admin__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.admin__nav-link--active {
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.1);
	}

	.admin__nav-link--back {
		color: var(--color-teal);
		margin-bottom: 0.5rem;
	}

	.admin__nav-link--header {
		justify-content: space-between;
	}

	:global(.admin__nav-caret) {
		transition: transform 200ms var(--ease-out);
	}

	:global(.admin__nav-caret--open) {
		transform: rotate(180deg);
	}

	.admin__nav-section {
		display: flex;
		flex-direction: column;
	}

	.admin__nav-submenu {
		display: flex;
		flex-direction: column;
	}

	.admin__nav-sublink {
		display: block;
		padding: 0.5rem 0.75rem 0.5rem 2.75rem;
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
		padding: 0.75rem;
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
		padding: 1rem;
	}

	/* Tablet breakpoint (768px+) */
	@media (min-width: 768px) {
		.admin {
			flex-direction: row;
		}

		.admin__mobile-header {
			display: none;
		}

		.admin__sidebar {
			position: sticky;
			transform: translateX(0);
		}

		.admin__overlay {
			display: none !important;
		}

		.admin__content {
			padding: 1.5rem;
		}
	}

	/* Desktop breakpoint (1024px+) */
	@media (min-width: 1024px) {
		.admin__content {
			padding: 2rem;
			max-width: 1400px;
		}
	}

	/* Admin Login */
	.admin-login {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		background: linear-gradient(145deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
	}

	.admin-login__card {
		width: 100%;
		max-width: 26rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 1.5rem;
	}

	.admin-login__header {
		text-align: center;
		margin-bottom: 1.5rem;
	}

	.admin-login__logo {
		display: inline-flex;
		gap: 0.35rem;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 0.75rem;
	}

	.admin-login__logo-brand {
		color: var(--color-white);
	}

	.admin-login__logo-accent {
		color: var(--color-teal);
	}

	.admin-login__badge {
		display: inline-block;
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: #f59e0b;
		background-color: rgba(245, 158, 11, 0.12);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 1rem;
	}

	.admin-login__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.admin-login__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.admin-login__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
		text-align: center;
	}

	.admin-login__form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.admin-login__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.admin-login__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.admin-login__input {
		width: 100%;
		padding: 0.75rem 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
		transition: border-color 200ms var(--ease-out);
	}

	.admin-login__input::placeholder {
		color: var(--color-grey-500);
	}

	.admin-login__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.admin-login__submit {
		width: 100%;
		padding: 0.85rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		transition:
			opacity 200ms var(--ease-out),
			transform 200ms var(--ease-out);
	}

	.admin-login__submit:hover:not(:disabled) {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	.admin-login__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.admin-login__forgot {
		display: block;
		text-align: center;
		margin-top: 1rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		text-decoration: none;
		font-weight: var(--w-medium);
		transition: opacity 200ms;
	}

	.admin-login__forgot:hover {
		opacity: 0.8;
	}

	.admin-login__back {
		display: block;
		text-align: center;
		margin-top: 0.75rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		transition: color 200ms;
	}

	.admin-login__back:hover {
		color: var(--color-teal);
	}

	@media (min-width: 480px) {
		.admin-login__card {
			padding: 2rem;
		}

		.admin-login__title {
			font-size: var(--fs-2xl);
		}

		.admin-login__form {
			gap: 1.25rem;
		}
	}

	@media (min-width: 768px) {
		.admin-login {
			padding: 2rem;
		}

		.admin-login__card {
			padding: 2.5rem;
		}
	}
</style>
