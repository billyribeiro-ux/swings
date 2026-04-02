<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AuthResponse } from '$lib/api/types';
	import { onMount } from 'svelte';
	import ChartBar from 'phosphor-svelte/lib/ChartBar';
	import Users from 'phosphor-svelte/lib/Users';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import Article from 'phosphor-svelte/lib/Article';
	import SignOut from 'phosphor-svelte/lib/SignOut';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';

	let { children } = $props();

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

			if (res.user.role !== 'admin') {
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

	/* Admin Login */
	.admin-login {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		background: linear-gradient(145deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
	}

	.admin-login__card {
		width: 100%;
		max-width: 26rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 2.5rem;
	}

	.admin-login__header {
		text-align: center;
		margin-bottom: 2rem;
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
		font-size: var(--fs-2xl);
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
		gap: 1.25rem;
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
</style>
