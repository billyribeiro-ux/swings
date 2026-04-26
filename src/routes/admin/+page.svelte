<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { AdminStats } from '$lib/api/types';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import TrendUpIcon from 'phosphor-svelte/lib/TrendUpIcon';
	import CalendarCheckIcon from 'phosphor-svelte/lib/CalendarCheckIcon';
	import CalendarBlankIcon from 'phosphor-svelte/lib/CalendarBlankIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';

	let stats = $state<AdminStats | null>(null);
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	let mounted = $state(false);

	onMount(async () => {
		try {
			stats = await api.get<AdminStats>('/api/admin/stats');
			loadError = null;
		} catch (e) {
			// Surface a real error state instead of silently rendering nothing —
			// the previous version showed neither the skeleton nor any data when
			// the API failed (e.g. 401 with stale JWT), which looked like the page
			// was stuck on the skeleton forever.
			loadError = e instanceof Error ? e.message : 'Failed to load dashboard stats';
			stats = null;
		} finally {
			loading = false;
			mounted = true;
		}
	});

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric'
		});
	}
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
		<span class="admin-dash__date-chip" aria-label="Reporting window: last 30 days">
			<CalendarBlankIcon size={14} weight="duotone" />
			<span>Last 30 days</span>
		</span>
	</header>

	{#if loading}
		<div class="admin-dash__skeleton-grid" aria-hidden="true">
			{#each Array(6) as _, i (i)}
				<div class="admin-dash__skeleton-card"></div>
			{/each}
		</div>
	{:else if loadError}
		<div class="admin-dash__error" role="alert">
			<p class="admin-dash__error-title">Couldn't load dashboard</p>
			<p class="admin-dash__error-body">{loadError}</p>
			<button
				type="button"
				class="admin-dash__error-retry"
				onclick={() => location.reload()}
			>
				Retry
			</button>
		</div>
	{:else if stats}
		<!-- KPI Cards -->
		<div class="admin-dash__kpis">
			<article class="kpi" style="--kpi-delay: 0ms;">
				<div class="kpi__head">
					<span class="kpi__label">Members</span>
					<span class="kpi__icon">
						<UsersIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">{stats.total_members.toLocaleString()}</p>
				<p class="kpi__hint">All registered accounts</p>
			</article>

			<article class="kpi" style="--kpi-delay: 60ms;">
				<div class="kpi__head">
					<span class="kpi__label">Subscribers</span>
					<span class="kpi__icon">
						<LightningIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">{stats.active_subscriptions.toLocaleString()}</p>
				<p class="kpi__hint">Currently paying customers</p>
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
				<p class="kpi__hint">Lifetime issues released</p>
			</article>

			<article class="kpi" style="--kpi-delay: 240ms;">
				<div class="kpi__head">
					<span class="kpi__label">Enrollments</span>
					<span class="kpi__icon">
						<BookOpenIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">{stats.total_enrollments.toLocaleString()}</p>
				<p class="kpi__hint">Across all courses</p>
			</article>

			<article class="kpi" style="--kpi-delay: 300ms;">
				<div class="kpi__head">
					<span class="kpi__label">Conversion</span>
					<span class="kpi__icon">
						<TrendUpIcon size={14} weight="duotone" />
					</span>
				</div>
				<p class="kpi__value">
					{stats.total_members > 0
						? ((stats.active_subscriptions / stats.total_members) * 100).toFixed(1)
						: '0.0'}<span class="kpi__unit">%</span>
				</p>
				<p class="kpi__hint">Members to active subscribers</p>
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
				<div class="admin-dash__empty">
					<span class="admin-dash__empty-icon" aria-hidden="true">
						<UsersIcon size={20} weight="duotone" />
					</span>
					<p class="admin-dash__empty-title">No members yet</p>
					<p class="admin-dash__empty-body">When people sign up, they'll appear here.</p>
				</div>
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
		letter-spacing: 0.08em;
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
		letter-spacing: -0.01em;
		margin: 0.25rem 0 0.5rem 0;
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
	}

	.admin-dash__date-chip :global(svg) {
		color: var(--color-grey-500);
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
		box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
		position: relative;
		transition:
			border-color 200ms var(--ease-out),
			background-color 200ms var(--ease-out);
	}

	.kpi:hover {
		border-color: rgba(255, 255, 255, 0.1);
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
	   EMPTY STATE
	   ==================================================================== */
	.admin-dash__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		min-height: 7rem;
		max-height: 9rem;
		padding: 1rem 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
		box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
		text-align: center;
	}

	.admin-dash__empty-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border-radius: var(--radius-md);
		background: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-500);
		margin-bottom: 0.125rem;
	}

	.admin-dash__empty-title {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-white);
		line-height: 1.35;
		margin: 0;
	}

	.admin-dash__empty-body {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--color-grey-500);
		margin: 0;
		line-height: 1.4;
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
		min-height: 2.5rem;
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
			transform 150ms var(--ease-out);
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

	/* Reduced motion — disable entrance animation */
	@media (prefers-reduced-motion: reduce) {
		.admin-dash--ready .kpi,
		.admin-dash--ready .admin-dash__section,
		.admin-dash__skeleton-card {
			animation: none;
			opacity: 1;
		}
	}
</style>
