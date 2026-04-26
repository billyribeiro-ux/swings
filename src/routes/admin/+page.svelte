<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteDate } from 'svelte/reactivity';
	import { browser } from '$app/environment';
	import { api } from '$lib/api/client';
	import type { AdminStats, DashboardRange } from '$lib/api/types';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import TrendUpIcon from 'phosphor-svelte/lib/TrendUpIcon';
	import TrendDownIcon from 'phosphor-svelte/lib/TrendDownIcon';
	import CalendarCheckIcon from 'phosphor-svelte/lib/CalendarCheckIcon';
	import CalendarBlankIcon from 'phosphor-svelte/lib/CalendarBlankIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CheckIcon from 'phosphor-svelte/lib/CheckIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';

	// ────────────────────────────────────────────────────────────────────
	// Range picker state
	//
	// The `range` lives in localStorage so navigating away and back doesn't
	// reset it — this is the same pattern Stripe / Linear use. Custom range
	// is split out so the chip label still reads "Custom" while the user is
	// editing the date inputs.
	// ────────────────────────────────────────────────────────────────────
	const RANGE_STORAGE_KEY = 'admin-dashboard-range';
	const RANGE_OPTIONS: { key: DashboardRange; label: string }[] = [
		{ key: 'last_7_days', label: 'Last 7 days' },
		{ key: 'last_30_days', label: 'Last 30 days' },
		{ key: 'last_90_days', label: 'Last 90 days' },
		{ key: 'year_to_date', label: 'Year to date' }
	];

	function readStoredRange(): DashboardRange {
		if (!browser) return 'last_30_days';
		try {
			const v = localStorage.getItem(RANGE_STORAGE_KEY);
			if (
				v === 'last_7_days' ||
				v === 'last_30_days' ||
				v === 'last_90_days' ||
				v === 'year_to_date' ||
				v === 'custom'
			) {
				return v;
			}
		} catch {
			// localStorage may throw in private mode — fall back silently.
		}
		return 'last_30_days';
	}

	function persistRange(v: DashboardRange) {
		if (!browser) return;
		try {
			localStorage.setItem(RANGE_STORAGE_KEY, v);
		} catch {
			// best-effort
		}
	}

	let range = $state<DashboardRange>('last_30_days');
	let customFrom = $state<string>('');
	let customTo = $state<string>('');
	let menuOpen = $state(false);
	let customPanelOpen = $state(false);
	let menuButtonEl = $state<HTMLButtonElement | undefined>(undefined);
	let menuEl = $state<HTMLDivElement | undefined>(undefined);

	let stats = $state<AdminStats | null>(null);
	let loading = $state(true);
	let refetching = $state(false);
	let loadError = $state<string | null>(null);
	let mounted = $state(false);

	const selectedLabel = $derived.by(() => {
		if (range === 'custom') {
			if (customFrom && customTo) return `${customFrom} → ${customTo}`;
			return 'Custom range';
		}
		return RANGE_OPTIONS.find((o) => o.key === range)?.label ?? 'Last 30 days';
	});

	function buildQuery(r: DashboardRange): string {
		if (r === 'custom') {
			if (!customFrom || !customTo) return '';
			return `?range=custom&from=${encodeURIComponent(customFrom)}&to=${encodeURIComponent(
				customTo
			)}`;
		}
		return `?range=${r}`;
	}

	async function loadStats(r: DashboardRange) {
		const qs = buildQuery(r);
		// Custom range without complete dates: don't fire the request, just wait.
		if (r === 'custom' && !qs) return;

		// Initial load → full skeleton; subsequent → inline dim.
		if (stats === null) loading = true;
		else refetching = true;

		try {
			stats = await api.get<AdminStats>(`/api/admin/stats${qs}`);
			loadError = null;
		} catch (e) {
			loadError = e instanceof Error ? e.message : 'Failed to load dashboard stats';
			// Keep the previous stats on screen so the page doesn't blank out on
			// transient errors (matches Vercel/Linear behaviour).
		} finally {
			loading = false;
			refetching = false;
			mounted = true;
		}
	}

	function selectRange(r: DashboardRange) {
		range = r;
		persistRange(r);
		menuOpen = false;
		customPanelOpen = false;
		void loadStats(r);
	}

	function openCustom() {
		// Default the inputs to the currently-resolved window so the user
		// has a sensible starting point instead of an empty form.
		if (!customFrom || !customTo) {
			const today = new SvelteDate();
			const thirty = new SvelteDate(today.getTime());
			thirty.setDate(today.getDate() - 30);
			customFrom = thirty.toISOString().slice(0, 10);
			customTo = today.toISOString().slice(0, 10);
		}
		range = 'custom';
		menuOpen = false;
		customPanelOpen = true;
	}

	function applyCustom() {
		if (!customFrom || !customTo) return;
		persistRange('custom');
		customPanelOpen = false;
		void loadStats('custom');
	}

	function toggleMenu() {
		menuOpen = !menuOpen;
		if (menuOpen) customPanelOpen = false;
	}

	function onChipKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			toggleMenu();
		} else if (e.key === 'Escape') {
			menuOpen = false;
			customPanelOpen = false;
		}
	}

	function onMenuKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			menuOpen = false;
			menuButtonEl?.focus();
		}
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
			e.preventDefault();
			const items = menuEl?.querySelectorAll<HTMLElement>('[role="menuitem"]');
			if (!items || items.length === 0) return;
			const active = document.activeElement as HTMLElement | null;
			let idx = Array.from(items).indexOf(active as HTMLElement);
			if (idx === -1) idx = 0;
			else idx = e.key === 'ArrowDown' ? (idx + 1) % items.length : (idx - 1 + items.length) % items.length;
			items[idx]?.focus();
		}
	}

	function onDocumentClick(e: MouseEvent) {
		if (!menuOpen && !customPanelOpen) return;
		const target = e.target as Node;
		if (menuEl?.contains(target) || menuButtonEl?.contains(target)) return;
		menuOpen = false;
		// Don't auto-close the custom panel on outside click — the user may be
		// dragging the native date picker which dispatches outside-target events.
	}

	onMount(() => {
		range = readStoredRange();
		document.addEventListener('mousedown', onDocumentClick);
		void loadStats(range);
		return () => {
			document.removeEventListener('mousedown', onDocumentClick);
		};
	});

	// ────────────────────────────────────────────────────────────────────
	// Formatters / delta math
	// ────────────────────────────────────────────────────────────────────
	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric'
		});
	}

	type Delta = { kind: 'up' | 'down' | 'flat'; pctLabel: string };

	function computeDelta(current: number, previous: number): Delta {
		const diff = current - previous;
		const pct = previous === 0 ? (current === 0 ? 0 : 100) : (diff / Math.abs(previous)) * 100;
		const pctLabel = previous === 0 && current === 0 ? '0%' : `${Math.abs(pct).toFixed(1)}%`;
		return {
			kind: diff > 0 ? 'up' : diff < 0 ? 'down' : 'flat',
			pctLabel
		};
	}

	const deltas = $derived.by(() => {
		if (!stats) return null;
		const p = stats.period;
		const pp = stats.previous_period;
		const netSubs = p.new_subscriptions - p.canceled_subscriptions;
		const prevNetSubs = pp.new_subscriptions - pp.canceled_subscriptions;
		const conversion = p.new_members > 0 ? p.new_subscriptions / p.new_members : 0;
		const prevConversion = pp.new_members > 0 ? pp.new_subscriptions / pp.new_members : 0;
		return {
			members: computeDelta(p.new_members, pp.new_members),
			subscribers: computeDelta(netSubs, prevNetSubs),
			watchlists: computeDelta(p.new_watchlists, pp.new_watchlists),
			enrollments: computeDelta(p.new_enrollments, pp.new_enrollments),
			conversion: computeDelta(Math.round(conversion * 1000), Math.round(prevConversion * 1000)),
			conversionRate: conversion
		};
	});
</script>

<svelte:head>
	<title>Admin Dashboard - Precision Options Signals</title>
</svelte:head>

<div class="admin-dash" class:admin-dash--ready={mounted}>
	<header class="admin-dash__page-header">
		<div class="admin-dash__page-header-text">
			<span class="admin-dash__eyebrow">Overview</span>
			<h1 class="admin-dash__title">Admin Dashboard</h1>
			<p class="admin-dash__subtitle">
				A snapshot of members, subscriptions, watchlists, and engagement across the platform.
			</p>
		</div>

		<!-- Range picker chip ──────────────────────────────────────────── -->
		<div class="admin-dash__range">
			<button
				bind:this={menuButtonEl}
				type="button"
				class="admin-dash__date-chip"
				class:admin-dash__date-chip--open={menuOpen}
				aria-haspopup="menu"
				aria-expanded={menuOpen}
				aria-label={`Reporting window: ${selectedLabel}. Click to change.`}
				onclick={toggleMenu}
				onkeydown={onChipKeydown}
			>
				<CalendarBlankIcon size={14} weight="duotone" />
				<span class="admin-dash__date-chip-label">{selectedLabel}</span>
				<CaretDownIcon size={10} weight="bold" />
			</button>

			{#if menuOpen}
				<div
					bind:this={menuEl}
					class="admin-dash__menu"
					role="menu"
					tabindex="-1"
					aria-label="Date range"
					onkeydown={onMenuKeydown}
				>
					{#each RANGE_OPTIONS as opt (opt.key)}
						<button
							type="button"
							role="menuitem"
							class="admin-dash__menu-item"
							class:admin-dash__menu-item--active={range === opt.key}
							onclick={() => selectRange(opt.key)}
						>
							<span class="admin-dash__menu-check" aria-hidden="true">
								{#if range === opt.key}
									<CheckIcon size={12} weight="bold" />
								{/if}
							</span>
							<span>{opt.label}</span>
						</button>
					{/each}
					<div class="admin-dash__menu-divider" role="separator"></div>
					<button
						type="button"
						role="menuitem"
						class="admin-dash__menu-item"
						class:admin-dash__menu-item--active={range === 'custom'}
						onclick={openCustom}
					>
						<span class="admin-dash__menu-check" aria-hidden="true">
							{#if range === 'custom'}
								<CheckIcon size={12} weight="bold" />
							{/if}
						</span>
						<span>Custom…</span>
					</button>
				</div>
			{/if}

			{#if customPanelOpen}
				<div class="admin-dash__custom-panel" role="group" aria-label="Custom date range">
					<label class="admin-dash__custom-field">
						<span>From</span>
						<input
							type="date"
							bind:value={customFrom}
							max={customTo || undefined}
							class="admin-dash__custom-input"
						/>
					</label>
					<label class="admin-dash__custom-field">
						<span>To</span>
						<input
							type="date"
							bind:value={customTo}
							min={customFrom || undefined}
							class="admin-dash__custom-input"
						/>
					</label>
					<button
						type="button"
						class="admin-dash__custom-apply"
						onclick={applyCustom}
						disabled={!customFrom || !customTo}
					>
						Apply
					</button>
				</div>
			{/if}
		</div>
	</header>

	{#if loading}
		<div class="admin-dash__skeleton-grid" aria-hidden="true">
			{#each Array(6) as _, i (i)}
				<div class="admin-dash__skeleton-card"></div>
			{/each}
		</div>
	{:else if loadError && !stats}
		<div class="admin-dash__error" role="alert">
			<p class="admin-dash__error-title">Couldn't load dashboard</p>
			<p class="admin-dash__error-body">{loadError}</p>
			<button type="button" class="admin-dash__error-retry" onclick={() => loadStats(range)}>
				Retry
			</button>
		</div>
	{:else if stats}
		<!-- KPI Cards -->
		<div
			class="admin-dash__kpis"
			class:admin-dash__kpis--refetching={refetching}
			aria-busy={refetching}
		>
			<article class="kpi" style="--kpi-delay: 0ms;">
				<div class="kpi__head">
					<span class="kpi__label">Members</span>
					<span class="kpi__icon">
						<UsersIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">{stats.total_members.toLocaleString()}</p>
				{#if deltas}
					<div class="kpi__meta">
						<span
							class={[
								'kpi__delta',
								deltas.members.kind === 'up' && 'kpi__delta--up',
								deltas.members.kind === 'down' && 'kpi__delta--down',
								deltas.members.kind === 'flat' && 'kpi__delta--flat'
							]}
						>
							{#if deltas.members.kind === 'up'}
								<TrendUpIcon size={12} weight="bold" />
							{:else if deltas.members.kind === 'down'}
								<TrendDownIcon size={12} weight="bold" />
							{:else}
								<span class="kpi__delta-flat" aria-hidden="true">–</span>
							{/if}
							<span>{deltas.members.pctLabel}</span>
						</span>
						<span class="kpi__hint">
							{stats.period.new_members > 0 ? '+' : ''}{stats.period.new_members.toLocaleString()} this period
						</span>
					</div>
				{/if}
			</article>

			<article class="kpi" style="--kpi-delay: 60ms;">
				<div class="kpi__head">
					<span class="kpi__label">Subscribers</span>
					<span class="kpi__icon">
						<LightningIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">{stats.active_subscriptions.toLocaleString()}</p>
				{#if deltas}
					<div class="kpi__meta">
						<span
							class={[
								'kpi__delta',
								deltas.subscribers.kind === 'up' && 'kpi__delta--up',
								deltas.subscribers.kind === 'down' && 'kpi__delta--down',
								deltas.subscribers.kind === 'flat' && 'kpi__delta--flat'
							]}
						>
							{#if deltas.subscribers.kind === 'up'}
								<TrendUpIcon size={12} weight="bold" />
							{:else if deltas.subscribers.kind === 'down'}
								<TrendDownIcon size={12} weight="bold" />
							{:else}
								<span class="kpi__delta-flat" aria-hidden="true">–</span>
							{/if}
							<span>{deltas.subscribers.pctLabel}</span>
						</span>
						<span class="kpi__hint">
							{(() => {
								const net =
									stats.period.new_subscriptions - stats.period.canceled_subscriptions;
								return `Net ${net > 0 ? '+' : ''}${net.toLocaleString()} this period`;
							})()}
						</span>
					</div>
				{/if}
			</article>

			<article class="kpi" style="--kpi-delay: 120ms;">
				<div class="kpi__head">
					<span class="kpi__label">M / A plans</span>
					<span class="kpi__icon">
						<CalendarCheckIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">
					{stats.monthly_subscriptions.toLocaleString()}<span class="kpi__sep">/</span>{stats.annual_subscriptions.toLocaleString()}
				</p>
				<p class="kpi__hint">Plan distribution</p>
			</article>

			<article class="kpi" style="--kpi-delay: 180ms;">
				<div class="kpi__head">
					<span class="kpi__label">Watchlists</span>
					<span class="kpi__icon">
						<ListChecksIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">{stats.total_watchlists.toLocaleString()}</p>
				{#if deltas}
					<div class="kpi__meta">
						<span
							class={[
								'kpi__delta',
								deltas.watchlists.kind === 'up' && 'kpi__delta--up',
								deltas.watchlists.kind === 'down' && 'kpi__delta--down',
								deltas.watchlists.kind === 'flat' && 'kpi__delta--flat'
							]}
						>
							{#if deltas.watchlists.kind === 'up'}
								<TrendUpIcon size={12} weight="bold" />
							{:else if deltas.watchlists.kind === 'down'}
								<TrendDownIcon size={12} weight="bold" />
							{:else}
								<span class="kpi__delta-flat" aria-hidden="true">–</span>
							{/if}
							<span>{deltas.watchlists.pctLabel}</span>
						</span>
						<span class="kpi__hint">
							{stats.period.new_watchlists > 0 ? '+' : ''}{stats.period.new_watchlists.toLocaleString()} this period
						</span>
					</div>
				{/if}
			</article>

			<article class="kpi" style="--kpi-delay: 240ms;">
				<div class="kpi__head">
					<span class="kpi__label">Enrollments</span>
					<span class="kpi__icon">
						<BookOpenIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">{stats.total_enrollments.toLocaleString()}</p>
				{#if deltas}
					<div class="kpi__meta">
						<span
							class={[
								'kpi__delta',
								deltas.enrollments.kind === 'up' && 'kpi__delta--up',
								deltas.enrollments.kind === 'down' && 'kpi__delta--down',
								deltas.enrollments.kind === 'flat' && 'kpi__delta--flat'
							]}
						>
							{#if deltas.enrollments.kind === 'up'}
								<TrendUpIcon size={12} weight="bold" />
							{:else if deltas.enrollments.kind === 'down'}
								<TrendDownIcon size={12} weight="bold" />
							{:else}
								<span class="kpi__delta-flat" aria-hidden="true">–</span>
							{/if}
							<span>{deltas.enrollments.pctLabel}</span>
						</span>
						<span class="kpi__hint">
							{stats.period.new_enrollments > 0 ? '+' : ''}{stats.period.new_enrollments.toLocaleString()} this period
						</span>
					</div>
				{/if}
			</article>

			<article class="kpi" style="--kpi-delay: 300ms;">
				<div class="kpi__head">
					<span class="kpi__label">Conversion</span>
					<span class="kpi__icon">
						<TrendUpIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">
					{deltas ? (deltas.conversionRate * 100).toFixed(1) : '0.0'}<span class="kpi__unit">%</span>
				</p>
				{#if deltas}
					<div class="kpi__meta">
						<span
							class={[
								'kpi__delta',
								deltas.conversion.kind === 'up' && 'kpi__delta--up',
								deltas.conversion.kind === 'down' && 'kpi__delta--down',
								deltas.conversion.kind === 'flat' && 'kpi__delta--flat'
							]}
						>
							{#if deltas.conversion.kind === 'up'}
								<TrendUpIcon size={12} weight="bold" />
							{:else if deltas.conversion.kind === 'down'}
								<TrendDownIcon size={12} weight="bold" />
							{:else}
								<span class="kpi__delta-flat" aria-hidden="true">–</span>
							{/if}
							<span>{deltas.conversion.pctLabel}</span>
						</span>
						<span class="kpi__hint">Members → active subscribers, this period</span>
					</div>
				{/if}
			</article>
		</div>

		<!-- Recent Members -->
		<section class="admin-dash__section" style="--section-delay: 360ms;">
			<div class="admin-dash__section-header">
				<div>
					<span class="admin-dash__eyebrow admin-dash__eyebrow--inline">Members</span>
					<h2 class="admin-dash__section-title">Recent sign-ups</h2>
				</div>
				<a href="/admin/members" class="admin-dash__link">
					<span>View all</span>
					<ArrowRightIcon size={12} weight="bold" />
				</a>
			</div>

			{#if stats.recent_members.length === 0}
				<p class="admin-dash__empty">
					<UsersIcon size={14} weight="regular" aria-hidden="true" />
					<span>No members yet — sign-ups will appear here.</span>
				</p>
			{:else}
				<!-- Mobile: Card view -->
				<div class="admin-dash__cards">
					{#each stats.recent_members as member (member.id)}
						<div class="member-card">
							<div class="member-card__row">
								<span class="member-card__label">Name</span>
								<span class="member-card__value member-card__name">{member.name}</span>
							</div>
							<div class="member-card__row">
								<span class="member-card__label">Email</span>
								<span class="member-card__value">{member.email}</span>
							</div>
							<div class="member-card__row">
								<span class="member-card__label">Role</span>
								<span
									class={[
										'member-card__role',
										member.role === 'admin'
											? 'member-card__role--admin'
											: 'member-card__role--member'
									]}
								>
									{member.role}
								</span>
							</div>
							<div class="member-card__row">
								<span class="member-card__label">Joined</span>
								<span class="member-card__value">{formatDate(member.created_at)}</span>
							</div>
						</div>
					{/each}
				</div>
				<!-- Tablet+: Table view -->
				<div class="admin-dash__table-wrap">
					<table class="admin-table">
						<thead>
							<tr>
								<th>Name</th>
								<th>Email</th>
								<th>Role</th>
								<th>Joined</th>
							</tr>
						</thead>
						<tbody>
							{#each stats.recent_members as member (member.id)}
								<tr>
									<td class="admin-table__name">{member.name}</td>
									<td class="admin-table__muted">{member.email}</td>
									<td>
										<span
											class={[
												'admin-table__role',
												member.role === 'admin'
													? 'admin-table__role--admin'
													: 'admin-table__role--member'
											]}
										>
											{member.role}
										</span>
									</td>
									<td class="admin-table__muted">{formatDate(member.created_at)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>

		<!-- Quick Actions -->
		<section class="admin-dash__section" style="--section-delay: 420ms;">
			<div class="admin-dash__section-header">
				<div>
					<span class="admin-dash__eyebrow admin-dash__eyebrow--inline">Shortcuts</span>
					<h2 class="admin-dash__section-title">Quick Actions</h2>
				</div>
			</div>
			<div class="admin-dash__actions">
				<a href="/admin/watchlists/new" class="admin-dash__action admin-dash__action--primary">
					<PlusIcon size={16} weight="bold" />
					<span>Create New Watchlist</span>
				</a>
				<a href="/admin/members" class="admin-dash__action">
					<UsersIcon size={16} weight="duotone" />
					<span>Manage Members</span>
					<ArrowRightIcon size={14} weight="bold" class="admin-dash__action-arrow" />
				</a>
				<a href="/admin/blog" class="admin-dash__action">
					<BookOpenIcon size={16} weight="duotone" />
					<span>Manage Blog</span>
					<ArrowRightIcon size={14} weight="bold" class="admin-dash__action-arrow" />
				</a>
			</div>
		</section>
	{/if}
</div>

<style>
	/* ====================================================================
	   ENTRY ANIMATION — fade up cards/sections on mount
	   ==================================================================== */
	@keyframes fadeUp {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes shimmer {
		0% {
			background-position: -200% 0;
		}
		100% {
			background-position: 200% 0;
		}
	}

	/* ====================================================================
	   PAGE WRAPPER — center on ultra-wide
	   ==================================================================== */
	.admin-dash {
		max-width: 1200px;
		margin: 0 auto;
	}

	/* ====================================================================
	   PAGE HEADER
	   ==================================================================== */
	.admin-dash__page-header {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.75rem;
		margin-bottom: 1.75rem;
	}

	.admin-dash__page-header-text {
		display: flex;
		flex-direction: column;
		gap: 0;
		min-width: 0;
	}

	.admin-dash__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
	}

	.admin-dash__eyebrow--inline {
		display: block;
		margin-bottom: 0.25rem;
	}

	.admin-dash__title {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.2;
		letter-spacing: -0.015em;
		margin: 0.25rem 0 0.25rem 0;
	}

	.admin-dash__subtitle {
		font-size: 0.875rem;
		font-weight: 400;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
		hyphens: none;
		word-break: normal;
		overflow-wrap: normal;
	}

	/* ====================================================================
	   RANGE PICKER — chip + dropdown menu + custom panel
	   ==================================================================== */
	.admin-dash__range {
		position: relative;
		display: inline-flex;
		flex-direction: column;
		align-items: stretch;
	}

	.admin-dash__date-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		height: 2rem;
		padding: 0 0.75rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-full);
		color: var(--color-grey-300);
		font-size: 0.75rem;
		font-weight: 500;
		line-height: 1;
		white-space: nowrap;
		cursor: pointer;
		font-family: inherit;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			color 150ms var(--ease-out);
	}

	.admin-dash__date-chip:hover {
		background: rgba(255, 255, 255, 0.07);
		border-color: rgba(255, 255, 255, 0.14);
		color: var(--color-white);
	}

	.admin-dash__date-chip--open {
		background: rgba(255, 255, 255, 0.08);
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}

	.admin-dash__date-chip:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin-dash__date-chip :global(svg) {
		color: var(--color-grey-500);
	}

	.admin-dash__date-chip--open :global(svg),
	.admin-dash__date-chip:hover :global(svg) {
		color: var(--color-grey-300);
	}

	.admin-dash__date-chip-label {
		font-variant-numeric: tabular-nums;
	}

	.admin-dash__menu {
		position: absolute;
		top: calc(100% + 0.5rem);
		right: 0;
		z-index: 30;
		min-width: 12rem;
		display: flex;
		flex-direction: column;
		padding: 0.375rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
	}

	.admin-dash__menu-item {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.625rem;
		background: transparent;
		border: 0;
		border-radius: var(--radius-md);
		color: var(--color-grey-200);
		font-size: 0.8125rem;
		font-weight: 500;
		font-family: inherit;
		text-align: left;
		cursor: pointer;
		transition: background-color 120ms var(--ease-out);
	}

	.admin-dash__menu-item:hover,
	.admin-dash__menu-item:focus-visible {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-white);
		outline: none;
	}

	.admin-dash__menu-item--active {
		color: var(--color-white);
		background-color: rgba(15, 164, 175, 0.1);
	}

	.admin-dash__menu-item--active:hover,
	.admin-dash__menu-item--active:focus-visible {
		background-color: rgba(15, 164, 175, 0.16);
	}

	.admin-dash__menu-check {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 0.875rem;
		height: 0.875rem;
		color: var(--color-teal);
	}

	.admin-dash__menu-divider {
		height: 1px;
		background-color: rgba(255, 255, 255, 0.06);
		margin: 0.25rem 0.25rem;
	}

	.admin-dash__custom-panel {
		position: absolute;
		top: calc(100% + 0.5rem);
		right: 0;
		z-index: 29;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
		min-width: 14rem;
	}

	.admin-dash__custom-field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.admin-dash__custom-input {
		padding: 0.4rem 0.5rem;
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		color: var(--color-white);
		font-size: 0.8125rem;
		font-family: inherit;
		color-scheme: dark;
	}

	.admin-dash__custom-input:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 1px;
	}

	.admin-dash__custom-apply {
		align-self: flex-end;
		margin-top: 0.25rem;
		padding: 0.45rem 0.9rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		border: 0;
		border-radius: var(--radius-md);
		color: var(--color-white);
		font-size: 0.8125rem;
		font-weight: 600;
		font-family: inherit;
		cursor: pointer;
		box-shadow: 0 4px 12px -6px rgba(15, 164, 175, 0.55);
		transition:
			opacity 120ms var(--ease-out),
			transform 120ms var(--ease-out),
			box-shadow 120ms var(--ease-out);
	}

	.admin-dash__custom-apply:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 6px 16px -6px rgba(15, 164, 175, 0.65);
	}

	.admin-dash__custom-apply:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin-dash__custom-apply:disabled {
		opacity: 0.4;
		cursor: not-allowed;
		box-shadow: none;
	}

	@media (prefers-reduced-motion: reduce) {
		.admin-dash__custom-apply {
			transition: none;
		}
		.admin-dash__custom-apply:hover:not(:disabled) {
			transform: none;
		}
	}

	/* ====================================================================
	   SKELETON LOADER
	   ==================================================================== */
	.admin-dash__skeleton-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
		gap: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.admin-dash__skeleton-card {
		height: 6.5rem;
		border-radius: var(--radius-lg);
		background: linear-gradient(
			90deg,
			rgba(255, 255, 255, 0.03) 0%,
			rgba(255, 255, 255, 0.06) 50%,
			rgba(255, 255, 255, 0.03) 100%
		);
		background-size: 200% 100%;
		animation: shimmer 1.6s ease-in-out infinite;
	}

	/* ====================================================================
	   ERROR STATE
	   ==================================================================== */
	.admin-dash__error {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 1.25rem 1.5rem;
		background-color: rgba(239, 68, 68, 0.08);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: var(--radius-xl);
		color: #fca5a5;
		margin-bottom: 1.5rem;
	}

	.admin-dash__error-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		line-height: 1.3;
		margin: 0;
	}

	.admin-dash__error-body {
		font-size: 0.875rem;
		font-weight: 400;
		line-height: 1.5;
		margin: 0;
	}

	.admin-dash__error-retry {
		margin-top: 0.5rem;
		padding: 0.5rem 1rem;
		background-color: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.8125rem;
		font-weight: 600;
		cursor: pointer;
		transition: background-color 150ms var(--ease-out);
	}

	.admin-dash__error-retry:hover {
		background-color: rgba(255, 255, 255, 0.12);
	}

	/* ====================================================================
	   KPI CARDS
	   ==================================================================== */
	.admin-dash__kpis {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		transition: opacity 200ms var(--ease-out);
	}

	.admin-dash__kpis--refetching {
		opacity: 0.5;
	}

	.admin-dash--ready .kpi {
		opacity: 0;
		animation: fadeUp 480ms var(--ease-out) forwards;
		animation-delay: var(--kpi-delay, 0ms);
	}

	.kpi {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-height: 6.5rem;
		padding: 0.875rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.05) inset,
			0 1px 2px rgba(0, 0, 0, 0.18);
		position: relative;
		transition:
			transform 200ms var(--ease-out),
			border-color 200ms var(--ease-out),
			box-shadow 200ms var(--ease-out),
			background-color 200ms var(--ease-out);
	}

	.kpi:hover {
		border-color: rgba(15, 164, 175, 0.25);
		transform: translateY(-1px);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.07) inset,
			0 6px 18px -8px rgba(15, 164, 175, 0.35),
			0 1px 2px rgba(0, 0, 0, 0.2);
	}

	@media (prefers-reduced-motion: reduce) {
		.kpi {
			transition: none;
		}
		.kpi:hover {
			transform: none;
		}
	}

	.kpi__head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.kpi__label {
		flex: 1;
		min-width: 0;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1.2;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.kpi__icon {
		width: 1.625rem;
		height: 1.625rem;
		border-radius: var(--radius-md);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		color: var(--color-grey-500);
		background-color: rgba(255, 255, 255, 0.04);
	}

	.kpi__value {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.1;
		letter-spacing: -0.02em;
		font-variant-numeric: tabular-nums lining-nums;
		margin: 0;
		margin-top: auto;
	}

	.kpi__unit {
		font-size: 0.55em;
		font-weight: 600;
		color: var(--color-grey-400);
		margin-left: 0.15em;
		letter-spacing: 0;
	}

	.kpi__sep {
		display: inline-block;
		margin: 0 0.25rem;
		color: var(--color-grey-600);
		font-weight: 500;
	}

	.kpi__hint {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--color-grey-500);
		margin: 0.125rem 0 0 0;
		line-height: 1.4;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.kpi__meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
		margin-top: 0.125rem;
	}

	.kpi__delta {
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		padding: 0.15rem 0.4rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 700;
		line-height: 1;
		font-variant-numeric: tabular-nums;
		background-color: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-400);
	}

	.kpi__delta--up {
		background-color: rgba(15, 164, 175, 0.16);
		color: #4fd1c5;
	}

	.kpi__delta--down {
		background-color: rgba(239, 68, 68, 0.14);
		color: #fca5a5;
	}

	.kpi__delta--flat {
		color: var(--color-grey-500);
	}

	.kpi__delta-flat {
		display: inline-block;
		width: 12px;
		text-align: center;
		font-weight: 700;
	}

	.kpi__meta .kpi__hint {
		flex: 1;
		min-width: 0;
		margin: 0;
	}

	/* ====================================================================
	   SECTIONS
	   ==================================================================== */
	.admin-dash__section {
		margin-bottom: 1.5rem;
	}

	.admin-dash--ready .admin-dash__section {
		opacity: 0;
		animation: fadeUp 480ms var(--ease-out) forwards;
		animation-delay: var(--section-delay, 0ms);
	}

	.admin-dash__section-header {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		margin-bottom: 1rem;
		gap: 1rem;
	}

	.admin-dash__section-title {
		font-family: inherit;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		letter-spacing: -0.005em;
		margin: 0 0 1rem 0;
		line-height: 1.3;
	}

	.admin-dash__section-header .admin-dash__section-title {
		margin-bottom: 0;
	}

	.admin-dash__eyebrow--inline {
		font-size: 0.6875rem;
		letter-spacing: 0.06em;
		margin: 0 0 0.25rem 0;
	}

	.admin-dash__link {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.8125rem;
		color: var(--color-teal-light);
		font-weight: 600;
		text-decoration: none;
		white-space: nowrap;
		padding: 0.4rem 0.6rem;
		border-radius: var(--radius-md);
		transition: all 150ms var(--ease-out);
	}

	.admin-dash__link:hover {
		background: rgba(15, 164, 175, 0.1);
		color: var(--color-teal-light);
	}

	/* ====================================================================
	   EMPTY STATE — quiet inline note, not a full card
	   ==================================================================== */
	.admin-dash__empty {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.4rem 0.65rem;
		font-size: 0.75rem;
		color: var(--color-grey-500);
		line-height: 1.3;
		margin: 0;
	}

	/* ====================================================================
	   MEMBER CARDS (mobile)
	   ==================================================================== */
	.admin-dash__table-wrap {
		display: block;
	}

	.admin-table {
		display: none;
	}

	.admin-dash__cards {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.member-card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.875rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		transition: all 200ms var(--ease-out);
	}

	.member-card:hover {
		border-color: rgba(255, 255, 255, 0.1);
		transform: translateY(-1px);
	}

	.member-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.member-card__label {
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.member-card__value {
		font-size: 0.875rem;
		font-weight: 400;
		color: var(--color-grey-300);
		text-align: right;
		word-break: break-word;
	}

	.member-card__name {
		font-weight: 600;
		color: var(--color-white);
	}

	.member-card__role {
		font-size: 0.6875rem;
		font-weight: 600;
		letter-spacing: 0.04em;
		padding: 0.2rem 0.55rem;
		border-radius: var(--radius-full);
		text-transform: capitalize;
	}

	.member-card__role--admin {
		background-color: rgba(245, 158, 11, 0.12);
		color: #f59e0b;
	}

	.member-card__role--member {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}

	/* ====================================================================
	   QUICK ACTIONS
	   ==================================================================== */
	.admin-dash__actions {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.5rem;
	}

	.admin-dash__action {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.75rem;
		padding: 0 1rem;
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.8125rem;
		font-weight: 600;
		text-decoration: none;
		text-align: left;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			transform 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.admin-dash__action:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.admin-dash__action :global(.admin-dash__action-arrow) {
		margin-left: auto;
		opacity: 0.5;
		transition:
			transform 200ms var(--ease-out),
			opacity 200ms var(--ease-out);
	}

	.admin-dash__action:hover {
		background-color: rgba(255, 255, 255, 0.07);
		border-color: rgba(255, 255, 255, 0.14);
		transform: translateY(-1px);
	}

	.admin-dash__action:hover :global(.admin-dash__action-arrow) {
		transform: translateX(2px);
		opacity: 0.85;
	}

	.admin-dash__action--primary {
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		border-color: transparent;
		color: var(--color-white);
		box-shadow: 0 6px 16px -6px rgba(15, 164, 175, 0.55);
	}

	.admin-dash__action--primary:hover {
		background: linear-gradient(135deg, var(--color-teal-light), var(--color-teal));
		border-color: transparent;
		transform: translateY(-1px);
		box-shadow: 0 8px 20px -6px rgba(15, 164, 175, 0.65);
	}

	/* ====================================================================
	   RESPONSIVE — Tablet (>=768px): header row, table, command-tile actions
	   ==================================================================== */
	@media (min-width: 768px) {
		.admin-dash__page-header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
			margin-bottom: 2rem;
		}

		.admin-dash__page-header-text {
			gap: 0;
		}

		.admin-dash__kpis {
			gap: 0.75rem;
			margin-bottom: 2rem;
		}

		/* Cap KPI grid at 3 columns from 768–1279px so we get 3+3 instead of 5+1 */
		.admin-dash__kpis {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}

		.admin-dash__section {
			margin-bottom: 2rem;
		}

		/* Show table, hide cards */
		.admin-dash__table-wrap {
			overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-2xl);
			box-shadow:
				0 1px 0 rgba(255, 255, 255, 0.03) inset,
				0 12px 32px rgba(0, 0, 0, 0.18);
		}

		.admin-table {
			display: table;
			width: 100%;
			border-collapse: collapse;
		}

		.admin-dash__cards {
			display: none;
		}

		.admin-table thead {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.admin-table th {
			text-align: left;
			font-size: 0.6875rem;
			font-weight: 700;
			color: var(--color-grey-500);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			padding: 0.875rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.admin-table td {
			padding: 0.875rem 1rem;
			font-size: 0.875rem;
			font-weight: 400;
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		}

		.admin-table tbody tr {
			transition: background-color 150ms var(--ease-out);
		}

		.admin-table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.admin-table tbody tr:last-child td {
			border-bottom: none;
		}

		.admin-table__name {
			font-weight: 600;
			color: var(--color-white);
		}

		.admin-table__muted {
			color: var(--color-grey-400);
		}

		.admin-table__role {
			display: inline-block;
			font-size: 0.6875rem;
			font-weight: 600;
			letter-spacing: 0.04em;
			padding: 0.2rem 0.6rem;
			border-radius: var(--radius-full);
			text-transform: capitalize;
		}

		.admin-table__role--admin {
			background-color: rgba(245, 158, 11, 0.12);
			color: #f59e0b;
		}

		.admin-table__role--member {
			background-color: rgba(15, 164, 175, 0.12);
			color: var(--color-teal);
		}

		.admin-dash__actions {
			grid-template-columns: repeat(3, minmax(0, 1fr));
			gap: 0.5rem;
		}
	}

	/* ====================================================================
	   RESPONSIVE — Wide (>=1280px): 6 KPI columns in a single row
	   ==================================================================== */
	@media (min-width: 1280px) {
		.admin-dash__kpis {
			grid-template-columns: repeat(6, minmax(0, 1fr));
			gap: 0.75rem;
		}
	}

	/* Reduced motion — disable entrance animation, hover transforms, shimmer */
	@media (prefers-reduced-motion: reduce) {
		.admin-dash--ready .kpi,
		.admin-dash--ready .admin-dash__section,
		.admin-dash__skeleton-card {
			animation: none;
			opacity: 1;
		}

		.admin-dash__action,
		.admin-dash__action--primary,
		.member-card {
			transition: none;
		}

		.admin-dash__action:hover,
		.admin-dash__action--primary:hover,
		.member-card:hover {
			transform: none;
		}

		.admin-dash__action:hover :global(.admin-dash__action-arrow) {
			transform: none;
		}
	}
</style>
