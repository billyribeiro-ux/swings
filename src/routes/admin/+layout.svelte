<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount, untrack } from 'svelte';
	import type { Component } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import type { RouteId } from '$app/types';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AuthResponse, UserResponse } from '$lib/api/types';
	import ChartBarIcon from 'phosphor-svelte/lib/ChartBarIcon';
	import PresentationChartIcon from 'phosphor-svelte/lib/PresentationChartIcon';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';
	import ArticleIcon from 'phosphor-svelte/lib/ArticleIcon';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';
	import SignOutIcon from 'phosphor-svelte/lib/SignOutIcon';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import CaretDoubleLeftIcon from 'phosphor-svelte/lib/CaretDoubleLeftIcon';
	import CaretDoubleRightIcon from 'phosphor-svelte/lib/CaretDoubleRightIcon';
	import ListIcon from 'phosphor-svelte/lib/ListIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import GraduationCapIcon from 'phosphor-svelte/lib/GraduationCapIcon';
	import CreditCardIcon from 'phosphor-svelte/lib/CreditCardIcon';
	import TagIcon from 'phosphor-svelte/lib/TagIcon';
	import ChatCircleDotsIcon from 'phosphor-svelte/lib/ChatCircleDotsIcon';
	import GearIcon from 'phosphor-svelte/lib/GearIcon';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import ReceiptIcon from 'phosphor-svelte/lib/ReceiptIcon';
	import ArrowSquareOutIcon from 'phosphor-svelte/lib/ArrowSquareOutIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import BellIcon from 'phosphor-svelte/lib/BellIcon';
	import StackIcon from 'phosphor-svelte/lib/StackIcon';
	import CookieIcon from 'phosphor-svelte/lib/CookieIcon';
	import CommandPalette from '$lib/components/admin/CommandPalette.svelte';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';
	import { SITE } from '$lib/seo/config';
	import {
		blogAdminItems,
		consentAdminItems,
		courseAdminItems,
		couponAdminItems,
		notificationAdminItems,
		popupAdminItems,
		publicAdminRoutes,
		resolveAdminHref,
		subscriptionAdminItems
	} from '$lib/components/admin/admin-nav';

	let { children } = $props();

	let paletteOpen = $state(false);

	const SIDEBAR_COLLAPSE_KEY = 'admin-sidebar-collapsed';
	let sidebarCollapsed = $state(false);

	function handleGlobalKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}

	// One-shot mount work (keybindings + initial sidebar persistence). Using
	// `onMount` instead of `$effect` keeps this OUT of the reactive graph so a
	// later `sidebarCollapsed` read by another effect cannot retrigger this
	// branch. Cleanup is handled by the returned teardown.
	onMount(() => {
		if (browser) {
			sidebarCollapsed = localStorage.getItem(SIDEBAR_COLLAPSE_KEY) === '1';
		}
		window.addEventListener('keydown', handleGlobalKey);
		return () => window.removeEventListener('keydown', handleGlobalKey);
	});

	let mobileMenuOpen = $state(false);
	let blogSubmenuOpen = $state(false);
	let courseSubmenuOpen = $state(false);
	let subscriptionSubmenuOpen = $state(false);
	let couponSubmenuOpen = $state(false);
	let popupSubmenuOpen = $state(false);
	let notificationSubmenuOpen = $state(false);
	let consentSubmenuOpen = $state(false);

	function toggleSidebarCollapsed() {
		sidebarCollapsed = !sidebarCollapsed;
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem(SIDEBAR_COLLAPSE_KEY, sidebarCollapsed ? '1' : '0');
		}
		if (sidebarCollapsed) {
			blogSubmenuOpen = false;
			courseSubmenuOpen = false;
			subscriptionSubmenuOpen = false;
			couponSubmenuOpen = false;
			popupSubmenuOpen = false;
			notificationSubmenuOpen = false;
			consentSubmenuOpen = false;
		}
	}

	const isPublicRoute = $derived(publicAdminRoutes.some((r) => page.url.pathname.startsWith(r)));

	/** True only after /api/auth/me succeeds — avoids child pages firing admin APIs with stale localStorage JWTs. */
	let adminSessionReady = $state(false);
	let adminSessionCheckInFlight = false; // plain ref, NOT $state — guards a fetch lifecycle, not UI

	function validateAdminSession() {
		if (adminSessionCheckInFlight) return;
		if (!auth.isAuthenticated || !auth.isAdmin) {
			adminSessionReady = false;
			return;
		}
		adminSessionCheckInFlight = true;
		void (async () => {
			try {
				const me = await api.get<UserResponse>('/api/auth/me');
				auth.setUser(me);
				adminSessionReady = true;
			} catch {
				auth.logout();
				adminSessionReady = false;
			} finally {
				adminSessionCheckInFlight = false;
			}
		})();
	}

	onMount(() => {
		// Public admin routes (forgot-password, reset-password) don't need a session.
		if (untrack(() => isPublicRoute)) {
			adminSessionReady = true;
			return;
		}
		validateAdminSession();
	});

	// Read-only watcher: when `auth.user` becomes null (logout in this tab or
	// another), invalidate the readiness flag. CRITICAL: this effect must read
	// only `auth`-derived state and `isPublicRoute` — never `adminSessionReady`
	// — so the writes below cannot retrigger it.
	$effect(() => {
		const publicRoute = isPublicRoute;
		const authed = auth.isAuthenticated;
		const admin = auth.isAdmin;
		untrack(() => {
			if (publicRoute) {
				adminSessionReady = true;
			} else if (!authed || !admin) {
				adminSessionReady = false;
			}
			// If publicRoute=false and authed+admin=true, do nothing here —
			// onMount/validateAdminSession owns the transition to ready=true.
		});
	});

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

			auth.setUser(res.user);
			adminSessionReady = true;
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
		// BFF: `auth.logout()` is now async (it pings the server to clear
		// the cookies). We deliberately do not await — the UI already
		// re-renders the moment `user` flips to `null`, and a network
		// hiccup must not trap the user in the admin shell.
		void auth.logout();
	}

	type NavItem = {
		href: RouteId;
		label: string;
		// Phosphor icons are Svelte components with their own prop shape; we don't
		// re-introduce phosphor's prop type here because the Svelte 5 `Component`
		// generic only needs the static-import widened back to a Component.
		icon: Component<Record<string, unknown>, Record<string, never>, ''>;
	};
	const navItems: NavItem[] = [
		{ href: '/admin', label: 'Dashboard', icon: ChartBarIcon },
		{ href: '/admin/analytics', label: 'Analytics', icon: PresentationChartIcon },
		{ href: '/admin/members', label: 'Members', icon: UsersIcon },
		{ href: '/admin/watchlists', label: 'Watchlists', icon: ListChecksIcon },
		{ href: '/admin/author', label: 'Author Profile', icon: UserCircleIcon }
	];

	function prettifySegment(seg: string) {
		const decoded = decodeURIComponent(seg).replace(/-/g, ' ');
		return decoded.charAt(0).toUpperCase() + decoded.slice(1);
	}

	const breadcrumbs = $derived.by(() => {
		const segments = page.url.pathname.split('/').filter(Boolean);
		// Always start at /admin
		const crumbs: { href: string; label: string }[] = [{ href: '/admin', label: 'Admin' }];
		let path = '';
		for (const seg of segments) {
			path += `/${seg}`;
			if (path === '/admin') continue;
			crumbs.push({ href: path, label: prettifySegment(seg) });
		}
		return crumbs;
	});
</script>

{#if isPublicRoute}
	{@render children()}
{:else if !auth.isAuthenticated || !auth.isAdmin}
	<div class="admin-login">
		<div class="admin-login__card">
			<div class="admin-login__header">
				<a href={resolve('/')} class="admin-login__logo">
					<span class="admin-login__logo-brand">{SITE.logoBrandPrimary}</span>
					<span class="admin-login__logo-accent">{SITE.logoBrandAccent}</span>
				</a>
				<span class="admin-login__badge">Admin</span>
				<h1 class="admin-login__title">Admin Login</h1>
				<p class="admin-login__subtitle">
					Enter your credentials to access the admin panel
				</p>
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

			<a href={resolve('/admin/forgot-password')} class="admin-login__forgot">Forgot password?</a>
			<a href={resolve('/')} class="admin-login__back">
				<ArrowLeftIcon size={14} weight="bold" />
				<span>Back to site</span>
			</a>
		</div>
	</div>
{:else if !adminSessionReady}
	<!--
		CLS fix: render the same shell skeleton the real admin layout will
		occupy once `adminSessionReady` flips to true. The pre-fix branch
		rendered a centered login-style card which caused a ~0.30 CLS
		reflow when the validated shell took its place. The skeleton below
		uses the same sidebar + main-area grid so the layout box stays
		stable through the validate → ready transition.
	-->
	<div class="admin admin--validating" aria-busy="true" aria-live="polite">
		<aside class="admin__sidebar admin__sidebar--skeleton" aria-hidden="true"></aside>
		<main class="admin__main">
			<div class="admin__validating-status">
				<p class="admin__validating-text">Validating session…</p>
			</div>
		</main>
	</div>
{:else}
	<div class="admin">
		<!-- Mobile Menu Overlay -->
		{#if mobileMenuOpen}
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
			<Tooltip label={mobileMenuOpen ? 'Close menu' : 'Open menu'} placement="bottom">
				<button
					class="admin__menu-toggle"
					aria-label={mobileMenuOpen ? 'Close menu' : 'Open menu'}
					onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
				>
					{#if mobileMenuOpen}
						<XIcon size={24} weight="bold" />
					{:else}
						<ListIcon size={24} weight="bold" />
					{/if}
				</button>
			</Tooltip>
			<a href={resolve('/')} class="admin__mobile-logo">
				<span class="admin__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="admin__logo-accent">{SITE.logoBrandAccent}</span>
			</a>
			<span class="admin__badge admin__badge--mobile">Admin</span>
		</header>

		<aside
			class="admin__sidebar"
			class:admin__sidebar--open={mobileMenuOpen}
			class:admin__sidebar--collapsed={sidebarCollapsed}
		>
			<div class="admin__sidebar-top">
				<a
					href={resolve('/')}
					class="admin__logo"
					title={SITE.name}
					aria-label={SITE.name}
				>
					<span class="admin__logo-mark" aria-hidden="true">P</span>
					<span class="admin__logo-wordmark">
						<span class="admin__logo-brand">{SITE.logoBrandPrimary}</span>
						<span class="admin__logo-accent">{SITE.logoBrandAccent}</span>
					</span>
				</a>
				<div class="admin__sidebar-top-actions">
					<span class="admin__badge">Admin</span>
					<Tooltip
						label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
						placement={sidebarCollapsed ? 'right' : 'bottom'}
					>
						<button
							type="button"
							class="admin__sidebar-pin"
							onclick={toggleSidebarCollapsed}
							aria-pressed={sidebarCollapsed}
							aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
						>
							{#if sidebarCollapsed}
								<CaretDoubleRightIcon size={14} weight="bold" />
							{:else}
								<CaretDoubleLeftIcon size={14} weight="bold" />
							{/if}
						</button>
					</Tooltip>
				</div>
			</div>

			<nav class="admin__nav">
				<span class="admin__nav-eyebrow">Overview</span>
				{#each navItems as item (item.href)}
					<Tooltip label={item.label} placement="right" disabled={!sidebarCollapsed}>
						<a
							href={resolveAdminHref(item.href)}
							class="admin__nav-link"
							class:admin__nav-link--active={page.url.pathname === item.href}
							onclick={() => (mobileMenuOpen = false)}
						>
							<item.icon size={20} weight="duotone" />
							<span>{item.label}</span>
						</a>
					</Tooltip>
				{/each}

				<span class="admin__nav-eyebrow">Content</span>
				<div class="admin__nav-section">
					<Tooltip label="Blog" placement="right" disabled={!sidebarCollapsed}>
						<button
							type="button"
							class="admin__nav-link admin__nav-link--header"
							class:admin__nav-link--header-open={blogSubmenuOpen}
							aria-expanded={blogSubmenuOpen}
							aria-label="Blog"
							onclick={() => (blogSubmenuOpen = !blogSubmenuOpen)}
						>
							<ArticleIcon size={20} weight="duotone" />
							<span class="admin__nav-link-label">Blog</span>
							<CaretRightIcon
								size={11}
								weight="bold"
								class="admin__nav-caret{blogSubmenuOpen
									? ' admin__nav-caret--open'
									: ''}"
							/>
						</button>
					</Tooltip>
					{#if blogSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each blogAdminItems as item (item.href)}
								<a
									href={resolveAdminHref(item.href)}
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

				<div class="admin__nav-section">
					<Tooltip label="Courses" placement="right" disabled={!sidebarCollapsed}>
						<button
							type="button"
							class="admin__nav-link admin__nav-link--header"
							class:admin__nav-link--header-open={courseSubmenuOpen}
							aria-expanded={courseSubmenuOpen}
							aria-label="Courses"
							onclick={() => (courseSubmenuOpen = !courseSubmenuOpen)}
						>
							<GraduationCapIcon size={20} weight="duotone" />
							<span class="admin__nav-link-label">Courses</span>
							<CaretRightIcon
								size={11}
								weight="bold"
								class="admin__nav-caret{courseSubmenuOpen
									? ' admin__nav-caret--open'
									: ''}"
							/>
						</button>
					</Tooltip>
					{#if courseSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each courseAdminItems as item (item.href)}
								<a
									href={resolveAdminHref(item.href)}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										courseSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<div class="admin__nav-section">
					<Tooltip label="Subscriptions" placement="right" disabled={!sidebarCollapsed}>
						<button
							type="button"
							class="admin__nav-link admin__nav-link--header"
							class:admin__nav-link--header-open={subscriptionSubmenuOpen}
							aria-expanded={subscriptionSubmenuOpen}
							aria-label="Subscriptions"
							onclick={() => (subscriptionSubmenuOpen = !subscriptionSubmenuOpen)}
						>
							<CreditCardIcon size={20} weight="duotone" />
							<span class="admin__nav-link-label">Subscriptions</span>
							<CaretRightIcon
								size={11}
								weight="bold"
								class="admin__nav-caret{subscriptionSubmenuOpen
									? ' admin__nav-caret--open'
									: ''}"
							/>
						</button>
					</Tooltip>
					{#if subscriptionSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each subscriptionAdminItems as item (item.href)}
								<a
									href={resolveAdminHref(item.href)}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										subscriptionSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<div class="admin__nav-section">
					<Tooltip label="Coupons" placement="right" disabled={!sidebarCollapsed}>
						<button
							type="button"
							class="admin__nav-link admin__nav-link--header"
							class:admin__nav-link--header-open={couponSubmenuOpen}
							aria-expanded={couponSubmenuOpen}
							aria-label="Coupons"
							onclick={() => (couponSubmenuOpen = !couponSubmenuOpen)}
						>
							<TagIcon size={20} weight="duotone" />
							<span class="admin__nav-link-label">Coupons</span>
							<CaretRightIcon
								size={11}
								weight="bold"
								class="admin__nav-caret{couponSubmenuOpen
									? ' admin__nav-caret--open'
									: ''}"
							/>
						</button>
					</Tooltip>
					{#if couponSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each couponAdminItems as item (item.href)}
								<a
									href={resolveAdminHref(item.href)}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										couponSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<div class="admin__nav-section">
					<Tooltip label="Popups" placement="right" disabled={!sidebarCollapsed}>
						<button
							type="button"
							class="admin__nav-link admin__nav-link--header"
							class:admin__nav-link--header-open={popupSubmenuOpen}
							aria-expanded={popupSubmenuOpen}
							aria-label="Popups"
							onclick={() => (popupSubmenuOpen = !popupSubmenuOpen)}
						>
							<ChatCircleDotsIcon size={20} weight="duotone" />
							<span class="admin__nav-link-label">Popups</span>
							<CaretRightIcon
								size={11}
								weight="bold"
								class="admin__nav-caret{popupSubmenuOpen
									? ' admin__nav-caret--open'
									: ''}"
							/>
						</button>
					</Tooltip>
					{#if popupSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each popupAdminItems as item (item.href)}
								<a
									href={resolveAdminHref(item.href)}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										popupSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<span class="admin__nav-eyebrow">Operations</span>
				<Tooltip label="Orders" placement="right" disabled={!sidebarCollapsed}>
					<a
						href={resolve('/admin/orders')}
						class="admin__nav-link"
						class:admin__nav-link--active={page.url.pathname.startsWith(
							'/admin/orders'
						)}
						onclick={() => (mobileMenuOpen = false)}
						data-testid="nav-orders"
					>
						<ReceiptIcon size={20} weight="duotone" />
						<span>Orders</span>
					</a>
				</Tooltip>

				<div class="admin__nav-section">
					<Tooltip label="Notifications" placement="right" disabled={!sidebarCollapsed}>
						<button
							type="button"
							class="admin__nav-link admin__nav-link--header"
							class:admin__nav-link--header-open={notificationSubmenuOpen}
							aria-expanded={notificationSubmenuOpen}
							aria-label="Notifications"
							onclick={() => (notificationSubmenuOpen = !notificationSubmenuOpen)}
						>
							<BellIcon size={20} weight="duotone" />
							<span class="admin__nav-link-label">Notifications</span>
							<CaretRightIcon
								size={11}
								weight="bold"
								class="admin__nav-caret{notificationSubmenuOpen
									? ' admin__nav-caret--open'
									: ''}"
							/>
						</button>
					</Tooltip>
					{#if notificationSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each notificationAdminItems as item (item.href)}
								<a
									href={resolveAdminHref(item.href)}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										notificationSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<Tooltip label="Outbox" placement="right" disabled={!sidebarCollapsed}>
					<a
						href={resolve('/admin/outbox')}
						class="admin__nav-link"
						class:admin__nav-link--active={page.url.pathname.startsWith(
							'/admin/outbox'
						)}
						onclick={() => (mobileMenuOpen = false)}
						data-testid="nav-outbox"
					>
						<StackIcon size={20} weight="duotone" />
						<span>Outbox</span>
					</a>
				</Tooltip>

				<span class="admin__nav-eyebrow">Governance</span>
				<Tooltip label="Security" placement="right" disabled={!sidebarCollapsed}>
					<a
						href={resolve('/admin/security')}
						class="admin__nav-link"
						class:admin__nav-link--active={page.url.pathname.startsWith(
							'/admin/security'
						)}
						onclick={() => (mobileMenuOpen = false)}
						data-testid="nav-security"
					>
						<ShieldCheckIcon size={20} weight="duotone" />
						<span>Security</span>
					</a>
				</Tooltip>

				<div class="admin__nav-section">
					<Tooltip label="Consent" placement="right" disabled={!sidebarCollapsed}>
						<button
							type="button"
							class="admin__nav-link admin__nav-link--header"
							class:admin__nav-link--header-open={consentSubmenuOpen}
							aria-expanded={consentSubmenuOpen}
							aria-label="Consent"
							onclick={() => (consentSubmenuOpen = !consentSubmenuOpen)}
						>
							<CookieIcon size={20} weight="duotone" />
							<span class="admin__nav-link-label">Consent</span>
							<CaretRightIcon
								size={11}
								weight="bold"
								class="admin__nav-caret{consentSubmenuOpen
									? ' admin__nav-caret--open'
									: ''}"
							/>
						</button>
					</Tooltip>
					{#if consentSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each consentAdminItems as item (item.href)}
								<a
									href={resolveAdminHref(item.href)}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										consentSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<Tooltip label="Audit log" placement="right" disabled={!sidebarCollapsed}>
					<a
						href={resolve('/admin/audit')}
						class="admin__nav-link"
						class:admin__nav-link--active={page.url.pathname.startsWith('/admin/audit')}
						onclick={() => (mobileMenuOpen = false)}
						data-testid="nav-audit"
					>
						<EyeIcon size={20} weight="duotone" />
						<span>Audit log</span>
					</a>
				</Tooltip>

				<Tooltip label="DSAR" placement="right" disabled={!sidebarCollapsed}>
					<a
						href={resolve('/admin/dsar')}
						class="admin__nav-link"
						class:admin__nav-link--active={page.url.pathname.startsWith('/admin/dsar')}
						onclick={() => (mobileMenuOpen = false)}
						data-testid="nav-dsar"
					>
						<TrashIcon size={20} weight="duotone" />
						<span>DSAR</span>
					</a>
				</Tooltip>

				<Tooltip label="Settings" placement="right" disabled={!sidebarCollapsed}>
					<a
						href={resolve('/admin/settings')}
						class="admin__nav-link"
						class:admin__nav-link--active={page.url.pathname === '/admin/settings'}
						onclick={() => (mobileMenuOpen = false)}
					>
						<GearIcon size={20} weight="duotone" />
						<span>Settings</span>
					</a>
				</Tooltip>
			</nav>

			<div class="admin__sidebar-footer">
				<Tooltip label="Member Dashboard" placement="right" disabled={!sidebarCollapsed}>
					<a
						href={resolve('/dashboard')}
						class="admin__nav-link admin__nav-link--back"
						onclick={() => (mobileMenuOpen = false)}
					>
						<ArrowLeftIcon size={18} />
						<span>Member Dashboard</span>
					</a>
				</Tooltip>
				<Tooltip label="Sign out" placement="right">
					<button onclick={handleLogout} class="admin__logout" aria-label="Sign out">
						<SignOutIcon size={20} weight="duotone" />
						<span>Sign Out</span>
					</button>
				</Tooltip>
			</div>
		</aside>

		<div class="admin__main">
			<header class="admin__main-topbar">
				<div class="admin__shell-inner admin__shell-inner--topbar">
					<nav class="admin__breadcrumbs" aria-label="Breadcrumb">
						<ol class="admin__breadcrumbs-list">
							{#each breadcrumbs as crumb, i (crumb.href)}
								<li class="admin__breadcrumbs-item">
									{#if i === breadcrumbs.length - 1}
										<span class="admin__breadcrumbs-current" aria-current="page"
											>{crumb.label}</span
										>
									{:else}
										<!-- Breadcrumb hrefs are derived live from `page.url.pathname` (which already has `paths.base` applied), so they don't need a second `resolve()` pass — they are already real URLs, not route-id literals. -->
										<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
										<a href={crumb.href} class="admin__breadcrumbs-link"
											>{crumb.label}</a
										>
										<CaretRightIcon
											size={18}
											weight="bold"
											class="admin__breadcrumbs-sep"
										/>
									{/if}
								</li>
							{/each}
						</ol>
					</nav>
					<div class="admin__main-topbar-actions">
						<button
							type="button"
							class="admin__search-pill"
							onclick={() => (paletteOpen = true)}
							aria-label="Open command palette"
						>
							<MagnifyingGlassIcon size={16} weight="bold" />
							<span>Search</span>
							<kbd class="admin__search-pill-kbd">⌘K</kbd>
						</button>
						<Tooltip label="Open public site in same tab" placement="bottom">
							<a href={resolve('/')} class="admin__view-site">
								<ArrowSquareOutIcon size={16} weight="bold" />
								<span>View site</span>
							</a>
						</Tooltip>
					</div>
				</div>
			</header>
			<div class="admin__content">
				<div class="admin__shell-inner">
					{@render children()}
				</div>
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

	/* CLS guard: render the validating skeleton at the same physical
	   footprint as the real shell so the layout box doesn't jump when
	   `adminSessionReady` flips. Sidebar skeleton matches the desktop
	   sidebar width; the main column claims the remaining viewport. */
	.admin--validating {
		flex-direction: row;
	}

	.admin__sidebar--skeleton {
		display: none;
	}

	.admin__validating-status {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 60vh;
	}

	.admin__validating-text {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		margin: 0;
	}

	@media (min-width: 1024px) {
		.admin__sidebar--skeleton {
			display: block;
			width: 16rem;
			flex-shrink: 0;
			background-color: rgba(11, 29, 58, 0.85);
			border-right: 1px solid rgba(255, 255, 255, 0.08);
		}
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
		background-color: rgba(11, 29, 58, 0.85);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
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

	.admin__menu-toggle:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin__mobile-logo {
		display: flex;
		gap: 0.3rem;
		font-size: 1rem;
		font-weight: 700;
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
		background-color: rgba(11, 29, 58, 0.85);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border-right: 1px solid rgba(255, 255, 255, 0.08);
		padding: 1rem 0.75rem 1rem;
		display: flex;
		flex-direction: column;
		z-index: 50;
		transform: translateX(-100%);
		transition: transform 300ms var(--ease-out);
		overflow-x: hidden;
		overflow-y: auto;
		box-sizing: border-box;
	}

	.admin__sidebar--open {
		transform: translateX(0);
	}

	.admin__overlay {
		display: block;
	}

	.admin__sidebar-top {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		padding: 0 0.25rem;
		min-width: 0;
	}

	.admin__sidebar-top-actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		flex-wrap: nowrap;
		min-width: 0;
		padding-left: 0.125rem;
	}

	.admin__sidebar-pin {
		display: none;
		align-items: center;
		justify-content: center;
		width: 1.5rem;
		height: 1.5rem;
		padding: 0;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-md);
		background: rgba(255, 255, 255, 0.03);
		color: var(--color-grey-500);
		cursor: pointer;
		flex-shrink: 0;
		transition:
			background-color 150ms var(--ease-out),
			color 150ms var(--ease-out),
			border-color 150ms var(--ease-out);
	}

	.admin__sidebar-pin:hover {
		background: rgba(255, 255, 255, 0.08);
		border-color: rgba(255, 255, 255, 0.16);
		color: var(--color-white);
	}

	.admin__sidebar-pin:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin__main-topbar {
		display: none;
	}

	.admin__logo {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-family: var(--font-heading);
		text-decoration: none;
		min-width: 0;
		line-height: 1.1;
	}

	.admin__logo-mark {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		width: 1.5rem;
		height: 1.5rem;
		border-radius: var(--radius-md);
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		color: var(--color-white);
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: -0.02em;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.12) inset,
			0 4px 10px -4px rgba(15, 164, 175, 0.55);
	}

	.admin__logo-wordmark {
		display: inline-flex;
		flex-direction: column;
		min-width: 0;
		font-size: 0.875rem;
		font-weight: 700;
		line-height: 1.1;
		letter-spacing: -0.01em;
	}

	.admin__logo-brand {
		color: var(--color-white);
	}

	.admin__logo-accent {
		color: var(--color-teal);
	}

	.admin__logo .admin__logo-brand {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.admin__logo .admin__logo-accent {
		font-size: 0.625rem;
		font-weight: 700;
		color: var(--color-teal-light);
		text-transform: uppercase;
		letter-spacing: 0.1em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-top: 0.125rem;
	}

	.admin__badge {
		font-size: 0.625rem;
		font-weight: 700;
		color: #f59e0b;
		background-color: rgba(245, 158, 11, 0.12);
		padding: 0.125rem 0.4rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.admin__badge--mobile {
		display: none;
	}

	.admin__nav {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		flex: 1;
		min-width: 0;
	}

	.admin__nav-eyebrow {
		display: block;
		padding: 1rem 0.75rem 0.4rem;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
	}

	.admin__nav-eyebrow:first-child {
		padding-top: 0.25rem;
	}

	.admin__nav-link {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.5rem 0.625rem;
		min-height: 2.25rem;
		border-radius: var(--radius-md);
		color: var(--color-grey-300);
		font-size: 0.8125rem;
		font-weight: 500;
		text-decoration: none;
		cursor: pointer;
		background: none;
		border: none;
		width: 100%;
		text-align: left;
		transition:
			color 160ms var(--ease-out),
			background-color 160ms var(--ease-out);
	}

	.admin__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.admin__nav-link:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin__nav-link--active {
		color: var(--color-teal-light);
		font-weight: 600;
		background-color: rgba(15, 164, 175, 0.12);
	}

	.admin__nav-link--active:hover {
		color: var(--color-teal-light);
		background-color: rgba(15, 164, 175, 0.16);
	}

	.admin__nav-link--back {
		color: var(--color-teal);
		margin-bottom: 0.5rem;
	}

	.admin__nav-link--header {
		justify-content: flex-start;
	}

	.admin__nav-link-label {
		flex-shrink: 0;
	}

	:global(.admin__nav-caret) {
		margin-left: 0.4rem;
		flex-shrink: 0;
		color: var(--color-grey-600);
		opacity: 0.85;
		transition:
			transform 180ms var(--ease-out),
			color 160ms var(--ease-out),
			opacity 160ms var(--ease-out);
	}

	.admin__nav-link--header:hover :global(.admin__nav-caret) {
		color: var(--color-grey-300);
		opacity: 1;
	}

	.admin__nav-link--header-open :global(.admin__nav-caret) {
		color: var(--color-teal-light);
		opacity: 1;
	}

	:global(.admin__nav-caret--open) {
		transform: rotate(90deg);
	}

	@media (prefers-reduced-motion: reduce) {
		:global(.admin__nav-caret) {
			transition: none;
		}
	}

	.admin__nav-section {
		display: flex;
		flex-direction: column;
	}

	.admin__nav-submenu {
		display: flex;
		flex-direction: column;
	}

	.admin__nav-submenu {
		padding: 0.125rem 0 0.25rem;
		gap: 0.0625rem;
	}

	.admin__nav-sublink {
		display: block;
		padding: 0.4rem 0.625rem 0.4rem 2.4rem;
		color: var(--color-grey-400);
		font-size: 0.8125rem;
		font-weight: 500;
		text-decoration: none;
		border-radius: var(--radius-md);
		transition:
			color 160ms var(--ease-out),
			background-color 160ms var(--ease-out);
	}

	.admin__nav-sublink:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.04);
	}

	.admin__nav-sublink:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin__sidebar-footer {
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 1rem;
	}

	.admin__logout {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		width: 100%;
		padding: 0.5rem 0.625rem;
		min-height: 2.25rem;
		border-radius: var(--radius-md);
		color: var(--color-grey-300);
		font-size: 0.8125rem;
		font-weight: 500;
		background: none;
		border: none;
		cursor: pointer;
		transition:
			color 160ms var(--ease-out),
			background-color 160ms var(--ease-out);
	}

	.admin__logout:hover {
		color: #fca5a5;
		background-color: rgba(239, 68, 68, 0.08);
	}

	.admin__logout:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin__main {
		flex: 1 1 auto;
		min-width: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		/* Enables @container queries for the content column (Material 3 “window size class”–style adaptation independent of viewport). See: https://m3.material.io/foundations/layout/applying-layout/window-size-classes */
		container-type: inline-size;
		container-name: admin-main;
	}

	/**
	 * Shared content column: max width + horizontal centering (Material / responsive admin pattern).
	 * Padding uses clamp() for comfortable touch targets on narrow viewports.
	 */
	.admin__shell-inner {
		width: 100%;
		max-width: min(75rem, 100%);
		margin-inline: auto;
		padding-inline: clamp(1rem, 4vw, 1.5rem);
		box-sizing: border-box;
	}

	.admin__shell-inner--topbar {
		display: none;
	}

	.admin__content {
		flex: 1 1 auto;
		min-width: 0;
		padding-block: clamp(1rem, 3vw, 1.75rem);
		padding-inline: 0;
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
			top: 0;
			align-self: flex-start;
			height: 100vh;
			flex: 0 0 16rem;
			max-width: 16rem;
			width: 16rem;
			transform: translateX(0);
			transition:
				flex-basis 240ms var(--ease-out),
				max-width 240ms var(--ease-out),
				width 240ms var(--ease-out),
				padding 240ms var(--ease-out);
		}

		.admin__sidebar-pin {
			display: flex;
		}

		.admin__sidebar--collapsed {
			flex: 0 0 4.35rem;
			max-width: 4.35rem;
			width: 4.35rem;
			padding: 1rem 0.4rem;
			align-items: center;
		}

		.admin__sidebar--collapsed .admin__sidebar-top {
			align-items: center;
			padding: 0;
		}

		.admin__sidebar--collapsed .admin__sidebar-top-actions {
			justify-content: center;
			width: 100%;
		}

		.admin__sidebar--collapsed .admin__logo-wordmark,
		.admin__sidebar--collapsed .admin__badge {
			display: none;
		}

		.admin__sidebar--collapsed .admin__logo {
			justify-content: center;
		}

		.admin__sidebar--collapsed .admin__nav-link span,
		.admin__sidebar--collapsed .admin__logout span,
		.admin__sidebar--collapsed .admin__nav-link--back span {
			display: none;
		}

		.admin__sidebar--collapsed .admin__nav-eyebrow {
			display: none;
		}

		.admin__sidebar--collapsed .admin__nav-link,
		.admin__sidebar--collapsed .admin__logout {
			justify-content: center;
		}

		.admin__sidebar--collapsed :global(.admin__nav-caret) {
			display: none;
		}

		.admin__sidebar--collapsed .admin__nav-submenu {
			display: none;
		}

		.admin__overlay {
			display: none !important;
		}

		.admin__main-topbar {
			display: flex;
			justify-content: center;
			align-items: center;
			gap: 0;
			padding: 0;
			border-bottom: 1px solid rgba(255, 255, 255, 0.08);
			background: rgba(11, 29, 58, 0.7);
			backdrop-filter: blur(24px);
			-webkit-backdrop-filter: blur(24px);
			min-height: 3.25rem;
			box-sizing: border-box;
			position: sticky;
			top: 0;
			z-index: 20;
		}

		.admin__shell-inner--topbar {
			display: flex;
			justify-content: space-between;
			align-items: center;
			gap: 1rem;
			min-height: 3.25rem;
			padding-block: 0.65rem;
			min-width: 0;
		}

		.admin__main-topbar-actions {
			display: inline-flex;
			align-items: center;
			gap: 0.5rem;
			flex-shrink: 0;
			min-width: 0;
		}

		/* Trail grows across the top bar; actions keep intrinsic width (M3: prioritize readable navigation). */
		.admin__breadcrumbs {
			flex: 1 1 0;
			min-width: 0;
			overflow: hidden;
		}

		.admin__breadcrumbs-list {
			display: flex;
			flex-wrap: nowrap;
			align-items: center;
			gap: 0.55rem;
			list-style: none;
			padding: 0;
			margin: 0;
			min-width: 0;
			width: 100%;
			max-width: 100%;
		}

		.admin__breadcrumbs-item {
			display: inline-flex;
			align-items: center;
			gap: 0.55rem;
			font-size: 0.9375rem;
			font-weight: 500;
			line-height: 1.35;
			color: var(--color-grey-300);
			min-width: 0;
		}

		.admin__breadcrumbs-item:not(:last-child) {
			flex-shrink: 0;
		}

		.admin__breadcrumbs-item:last-child {
			flex: 1 1 0;
			min-width: 0;
			max-width: 100%;
		}

		.admin__breadcrumbs-link {
			color: var(--color-grey-400);
			font-weight: 500;
			text-decoration: none;
			padding: 0.3rem 0.45rem;
			margin: -0.3rem -0.45rem;
			border-radius: var(--radius-sm);
			min-height: 2.25rem;
			display: inline-flex;
			align-items: center;
			transition:
				color 150ms var(--ease-out),
				background-color 150ms var(--ease-out);
			white-space: nowrap;
		}

		.admin__breadcrumbs-link:hover {
			color: var(--color-white);
			background-color: rgba(255, 255, 255, 0.04);
		}

		.admin__breadcrumbs-link:focus-visible {
			outline: 2px solid var(--color-teal);
			outline-offset: 2px;
		}

		.admin__breadcrumbs-current {
			display: block;
			color: var(--color-white);
			font-weight: 600;
			font-size: 0.9375rem;
			line-height: 1.35;
			letter-spacing: -0.005em;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
			min-width: 0;
		}

		:global(.admin__breadcrumbs-sep) {
			color: var(--color-grey-600);
			flex-shrink: 0;
			opacity: 0.85;
		}

		/*
		 * Compact main column (<600px): Material “compact” window class — avoid crushing the trail
		 * beside actions; stack full-width rows (m3.material.io window size classes).
		 */
		@container admin-main (max-width: 599.98px) {
			.admin__shell-inner--topbar {
				flex-wrap: wrap;
				align-items: flex-start;
				row-gap: 0.625rem;
				column-gap: 0.75rem;
			}

			.admin__breadcrumbs {
				flex: 1 1 100%;
				order: 1;
				overflow: visible;
			}

			.admin__breadcrumbs-list {
				flex-wrap: wrap;
				row-gap: 0.35rem;
			}

			.admin__main-topbar-actions {
				order: 2;
				width: 100%;
				justify-content: flex-end;
				flex-wrap: wrap;
			}
		}

		.admin__search-pill,
		.admin__view-site {
			box-sizing: border-box;
			display: inline-flex;
			align-items: center;
			gap: 0.5rem;
			height: 2.25rem;
			padding: 0 0.75rem;
			background-color: rgba(255, 255, 255, 0.04);
			border: 1px solid rgba(255, 255, 255, 0.08);
			border-radius: var(--radius-lg);
			font-size: 0.8125rem;
			font-weight: 500;
			line-height: 1;
			text-decoration: none;
			cursor: pointer;
			transition:
				background-color 150ms var(--ease-out),
				border-color 150ms var(--ease-out),
				color 150ms var(--ease-out);
		}

		.admin__search-pill {
			color: var(--color-grey-300);
		}

		.admin__view-site {
			color: var(--color-white);
			font-weight: 600;
		}

		.admin__search-pill:hover,
		.admin__view-site:hover {
			background-color: rgba(255, 255, 255, 0.08);
			border-color: rgba(255, 255, 255, 0.16);
			color: var(--color-white);
		}

		.admin__search-pill:focus-visible,
		.admin__view-site:focus-visible {
			outline: 2px solid var(--color-teal);
			outline-offset: 2px;
		}

		.admin__search-pill-kbd {
			display: inline-flex;
			align-items: center;
			padding: 0.05rem 0.4rem;
			margin-left: 0.15rem;
			font-family: var(--font-ui);
			font-size: 0.6875rem;
			font-weight: 600;
			color: var(--color-grey-400);
			background: rgba(255, 255, 255, 0.06);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: var(--radius-md);
			line-height: 1.4;
		}
	}

	/* Desktop breakpoint (1024px+): slightly more air; column stays centered via `.admin__shell-inner`. */
	@media (min-width: 1024px) {
		.admin__shell-inner {
			padding-inline: clamp(1.5rem, 3vw, 2.25rem);
		}

		.admin__content {
			padding-block: clamp(1.35rem, 2.5vw, 2.25rem);
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
		position: relative;
		overflow: hidden;
	}

	.admin-login::before {
		content: '';
		position: absolute;
		top: -20%;
		left: -10%;
		width: 50%;
		height: 50%;
		background: radial-gradient(circle, rgba(15, 164, 175, 0.25) 0%, transparent 70%);
		filter: blur(80px);
		z-index: 0;
		animation: pulse-glow-slow 8s ease-in-out infinite alternate;
	}

	.admin-login::after {
		content: '';
		position: absolute;
		bottom: -20%;
		right: -10%;
		width: 50%;
		height: 50%;
		background: radial-gradient(circle, rgba(212, 168, 67, 0.15) 0%, transparent 70%);
		filter: blur(80px);
		z-index: 0;
		animation: pulse-glow-slow 10s ease-in-out infinite alternate-reverse;
	}

	@keyframes pulse-glow-slow {
		0% {
			transform: scale(1) translate(0, 0);
			opacity: 0.8;
		}
		100% {
			transform: scale(1.1) translate(5%, 5%);
			opacity: 1;
		}
	}

	.admin-login__card {
		width: 100%;
		max-width: 26rem;
		background-color: rgba(19, 43, 80, 0.4);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		padding: 1.5rem;
		position: relative;
		z-index: 1;
		box-shadow:
			0 25px 50px -12px rgba(0, 0, 0, 0.5),
			0 0 0 1px rgba(255, 255, 255, 0.05) inset;
	}

	.admin-login__header {
		text-align: center;
		margin-bottom: 1.5rem;
	}

	.admin-login__logo {
		display: inline-flex;
		gap: 0.35rem;
		font-size: 1.25rem;
		font-weight: 700;
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
		font-size: 0.6875rem;
		font-weight: 700;
		color: #f59e0b;
		background-color: rgba(245, 158, 11, 0.12);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin-bottom: 1rem;
	}

	.admin-login__title {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		font-family: var(--font-heading);
		letter-spacing: -0.01em;
		line-height: 1.2;
		margin-bottom: 0.5rem;
	}

	.admin-login__subtitle {
		color: var(--color-grey-400);
		font-size: 0.875rem;
		line-height: 1.5;
	}

	.admin-login__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		line-height: 1.5;
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
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--color-white);
	}

	.admin-login__input {
		width: 100%;
		padding: 0.75rem 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.875rem;
		font-weight: 400;
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
		font-weight: 600;
		font-size: 0.8125rem;
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
		font-size: 0.875rem;
		text-decoration: none;
		font-weight: 500;
		transition: opacity 200ms;
	}

	.admin-login__forgot:hover {
		opacity: 0.8;
	}

	.admin-login__back {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		width: 100%;
		text-align: center;
		margin-top: 0.75rem;
		color: var(--color-grey-400);
		font-size: 0.75rem;
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
