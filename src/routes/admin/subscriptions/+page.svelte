<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import CaretLeft from 'phosphor-svelte/lib/CaretLeft';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import CurrencyDollar from 'phosphor-svelte/lib/CurrencyDollar';
	import Users from 'phosphor-svelte/lib/Users';
	import CalendarCheck from 'phosphor-svelte/lib/CalendarCheck';
	import Repeat from 'phosphor-svelte/lib/Repeat';
	import Funnel from 'phosphor-svelte/lib/Funnel';

	interface Subscription {
		id: string;
		member_id: string;
		member_name: string;
		member_email: string;
		plan_name: string;
		status: 'active' | 'canceled' | 'past_due';
		interval: 'month' | 'year';
		amount_cents: number;
		start_date: string;
		next_renewal: string | null;
	}

	interface SubscriptionStats {
		total_active: number;
		monthly_count: number;
		annual_count: number;
		mrr_cents: number;
	}

	interface PaginatedSubscriptions {
		data: Subscription[];
		total: number;
		total_pages: number;
		page: number;
	}

	let subscriptions = $state<Subscription[]>([]);
	let stats = $state<SubscriptionStats | null>(null);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let statsLoading = $state(true);
	let search = $state('');
	let statusFilter = $state<'all' | 'active' | 'canceled' | 'past_due'>('all');

	let searchTimeout: ReturnType<typeof setTimeout>;

	function handleSearchInput(value: string) {
		search = value;
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			page = 1;
			loadSubscriptions();
		}, 300);
	}

	function handleFilterChange(value: string) {
		statusFilter = value as typeof statusFilter;
		page = 1;
		loadSubscriptions();
	}

	async function loadStats() {
		statsLoading = true;
		try {
			stats = await api.get<SubscriptionStats>('/api/admin/subscriptions/stats');
		} catch {
			stats = null;
		} finally {
			statsLoading = false;
		}
	}

	async function loadSubscriptions() {
		loading = true;
		try {
			const params = new URLSearchParams({ page: String(page), per_page: '15' });
			if (search.trim()) params.set('search', search.trim());
			if (statusFilter !== 'all') params.set('status', statusFilter);

			const res = await api.get<PaginatedSubscriptions>(
				`/api/admin/subscriptions?${params.toString()}`
			);
			subscriptions = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch {
			subscriptions = [];
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadStats();
		loadSubscriptions();
	});

	function formatMoney(cents: number): string {
		return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function statusLabel(status: string): string {
		if (status === 'past_due') return 'Past Due';
		return status.charAt(0).toUpperCase() + status.slice(1);
	}

	function navigateToMember(memberId: string) {
		goto(`/admin/members/${memberId}`);
	}
</script>

<svelte:head>
	<title>Subscriptions - Admin</title>
</svelte:head>

<div class="subs-page">
	<div class="subs-page__header">
		<div>
			<h1 class="subs-page__title">Subscriptions</h1>
			<p class="subs-page__count">{total} total subscriptions</p>
		</div>
		<a href="/admin/subscriptions/plans" class="subs-page__plans-link">Manage Plans</a>
	</div>

	{#if statsLoading}
		<div class="subs-page__kpis">
			{#each Array(4) as _, i (i)}
				<div class="kpi kpi--skeleton">
					<div class="kpi__icon-skeleton"></div>
					<div class="kpi__text-skeleton">
						<div class="skeleton-line skeleton-line--short"></div>
						<div class="skeleton-line skeleton-line--long"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if stats}
		<div class="subs-page__kpis">
			<div class="kpi">
				<div class="kpi__icon kpi__icon--green">
					<Users size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Total Active</p>
					<p class="kpi__value">{stats.total_active.toLocaleString()}</p>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--blue">
					<CalendarCheck size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Monthly</p>
					<p class="kpi__value">{stats.monthly_count.toLocaleString()}</p>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--purple">
					<Repeat size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">Annual</p>
					<p class="kpi__value">{stats.annual_count.toLocaleString()}</p>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--teal">
					<CurrencyDollar size={22} weight="fill" />
				</div>
				<div>
					<p class="kpi__label">MRR</p>
					<p class="kpi__value">{formatMoney(stats.mrr_cents)}</p>
				</div>
			</div>
		</div>
	{/if}

	<div class="subs-page__filters">
		<div class="subs-page__search">
			<MagnifyingGlass size={18} weight="bold" />
			<input
				type="text"
				placeholder="Search by name or email..."
				value={search}
				oninput={(e) => handleSearchInput(e.currentTarget.value)}
				class="subs-page__search-input"
			/>
		</div>
		<div class="subs-page__filter-group">
			<Funnel size={16} weight="bold" />
			<select
				value={statusFilter}
				onchange={(e) => handleFilterChange(e.currentTarget.value)}
				class="subs-page__select"
			>
				<option value="all">All Statuses</option>
				<option value="active">Active</option>
				<option value="canceled">Canceled</option>
				<option value="past_due">Past Due</option>
			</select>
		</div>
	</div>

	{#if loading}
		<div class="subs-page__skeleton-list">
			{#each Array(6) as _, i (i)}
				<div class="skeleton-row">
					<div class="skeleton-line skeleton-line--full"></div>
				</div>
			{/each}
		</div>
	{:else if subscriptions.length === 0}
		<div class="subs-page__empty">
			<p>No subscriptions found.</p>
		</div>
	{:else}
		<div class="subs-page__cards">
			{#each subscriptions as sub (sub.id)}
				<button type="button" class="sub-card" onclick={() => navigateToMember(sub.member_id)}>
					<div class="sub-card__header">
						<div class="sub-card__member">
							<span class="sub-card__name">{sub.member_name}</span>
							<span class="sub-card__email">{sub.member_email}</span>
						</div>
						<span class="sub-card__status sub-card__status--{sub.status}">
							{statusLabel(sub.status)}
						</span>
					</div>
					<div class="sub-card__details">
						<div class="sub-card__row">
							<span class="sub-card__label">Plan</span>
							<span class="sub-card__plan-badge">{sub.plan_name}</span>
						</div>
						<div class="sub-card__row">
							<span class="sub-card__label">Amount</span>
							<span class="sub-card__value">{formatMoney(sub.amount_cents)}/{sub.interval === 'month' ? 'mo' : 'yr'}</span>
						</div>
						<div class="sub-card__row">
							<span class="sub-card__label">Started</span>
							<span class="sub-card__value">{formatDate(sub.start_date)}</span>
						</div>
						{#if sub.next_renewal}
							<div class="sub-card__row">
								<span class="sub-card__label">Renews</span>
								<span class="sub-card__value">{formatDate(sub.next_renewal)}</span>
							</div>
						{/if}
					</div>
				</button>
			{/each}
		</div>

		<div class="subs-page__table-wrap">
			<table class="s-table">
				<thead>
					<tr>
						<th>Member</th>
						<th>Plan</th>
						<th>Status</th>
						<th>Amount</th>
						<th>Start Date</th>
						<th>Next Renewal</th>
					</tr>
				</thead>
				<tbody>
					{#each subscriptions as sub (sub.id)}
						<tr class="s-table__row--clickable" onclick={() => navigateToMember(sub.member_id)}>
							<td>
								<div class="s-table__member">
									<span class="s-table__name">{sub.member_name}</span>
									<span class="s-table__email">{sub.member_email}</span>
								</div>
							</td>
							<td><span class="s-table__plan-badge">{sub.plan_name}</span></td>
							<td>
								<span class="s-table__status s-table__status--{sub.status}">
									{statusLabel(sub.status)}
								</span>
							</td>
							<td class="s-table__amount">
								{formatMoney(sub.amount_cents)}<span class="s-table__interval">/{sub.interval === 'month' ? 'mo' : 'yr'}</span>
							</td>
							<td>{formatDate(sub.start_date)}</td>
							<td>{sub.next_renewal ? formatDate(sub.next_renewal) : '\u2014'}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if totalPages > 1}
		<div class="subs-page__pagination">
			<button
				onclick={() => { page--; loadSubscriptions(); }}
				disabled={page <= 1}
				class="subs-page__page-btn"
			>
				<CaretLeft size={16} weight="bold" /> Prev
			</button>
			<span class="subs-page__page-info">Page {page} of {totalPages}</span>
			<button
				onclick={() => { page++; loadSubscriptions(); }}
				disabled={page >= totalPages}
				class="subs-page__page-btn"
			>
				Next <CaretRight size={16} weight="bold" />
			</button>
		</div>
	{/if}
</div>

<style>
	.subs-page__header {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}
	.subs-page__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.subs-page__count {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-top: 0.15rem;
	}
	.subs-page__plans-link {
		display: inline-flex;
		align-items: center;
		align-self: flex-start;
		padding: 0.55rem 1rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		border-radius: var(--radius-lg);
		text-decoration: none;
		transition: opacity var(--duration-200) var(--ease-out), transform var(--duration-200) var(--ease-out);
	}
	.subs-page__plans-link:hover { opacity: 0.9; transform: translateY(-1px); }

	/* KPI */
	.subs-page__kpis {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}
	.kpi {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}
	.kpi--skeleton { min-height: 4rem; }
	.kpi__icon-skeleton {
		width: 2.5rem; height: 2.5rem;
		border-radius: var(--radius-lg);
		background: rgba(255, 255, 255, 0.06);
		animation: shimmer 1.5s infinite;
		flex-shrink: 0;
	}
	.kpi__text-skeleton { display: flex; flex-direction: column; gap: 0.4rem; flex: 1; }
	.skeleton-line {
		height: 0.75rem;
		border-radius: var(--radius-sm);
		background: rgba(255, 255, 255, 0.06);
		animation: shimmer 1.5s infinite;
	}
	.skeleton-line--short { width: 40%; }
	.skeleton-line--long { width: 65%; height: 1rem; }
	.skeleton-line--full { width: 100%; height: 2.5rem; }
	@keyframes shimmer { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.8; } }
	.kpi__icon {
		width: 2.5rem; height: 2.5rem;
		border-radius: var(--radius-lg);
		display: flex; align-items: center; justify-content: center;
		flex-shrink: 0;
	}
	.kpi__icon--green { background-color: rgba(34, 197, 94, 0.15); color: #22c55e; }
	.kpi__icon--blue { background-color: rgba(59, 130, 246, 0.15); color: #3b82f6; }
	.kpi__icon--purple { background-color: rgba(168, 85, 247, 0.15); color: #a855f7; }
	.kpi__icon--teal { background-color: rgba(15, 164, 175, 0.15); color: var(--color-teal); }
	.kpi__label { font-size: var(--fs-xs); color: var(--color-grey-400); margin-bottom: 0.1rem; }
	.kpi__value { font-size: var(--fs-md); font-weight: var(--w-bold); color: var(--color-white); }

	/* Filters */
	.subs-page__filters { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.25rem; }
	.subs-page__search {
		display: flex; align-items: center; gap: 0.5rem;
		padding: 0.65rem 0.85rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.subs-page__search:focus-within { border-color: var(--color-teal); }
	.subs-page__search-input {
		flex: 1; background: none; border: none; outline: none;
		color: var(--color-white); font-size: var(--fs-sm); font-family: var(--font-ui);
	}
	.subs-page__search-input::placeholder { color: var(--color-grey-500); }
	.subs-page__filter-group { display: flex; align-items: center; gap: 0.5rem; color: var(--color-grey-400); }
	.subs-page__select {
		flex: 1; padding: 0.65rem 0.85rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white); font-size: var(--fs-sm); font-family: var(--font-ui);
		cursor: pointer; appearance: auto;
	}
	.subs-page__select option { background-color: var(--color-navy-mid); color: var(--color-white); }

	/* Skeleton/Empty */
	.subs-page__skeleton-list { display: flex; flex-direction: column; gap: 0.5rem; }
	.skeleton-row {
		padding: 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
	}
	.subs-page__empty {
		text-align: center; padding: 3rem 1rem;
		color: var(--color-grey-400); font-size: var(--fs-sm);
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}

	/* Mobile Card View */
	.subs-page__cards { display: flex; flex-direction: column; gap: 0.5rem; }
	.subs-page__table-wrap { display: none; }
	.sub-card {
		display: flex; flex-direction: column; gap: 0.65rem;
		padding: 0.85rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
		cursor: pointer; text-align: left; width: 100%;
		font-family: var(--font-ui);
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.sub-card:hover { border-color: rgba(15, 164, 175, 0.25); }
	.sub-card__header { display: flex; justify-content: space-between; align-items: flex-start; gap: 0.5rem; }
	.sub-card__member { display: flex; flex-direction: column; gap: 0.1rem; min-width: 0; }
	.sub-card__name { font-weight: var(--w-semibold); color: var(--color-white); font-size: var(--fs-sm); }
	.sub-card__email { font-size: var(--fs-xs); color: var(--color-grey-400); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.sub-card__status {
		font-size: var(--fs-xs); font-weight: var(--w-semibold);
		padding: 0.2rem 0.6rem; border-radius: var(--radius-full);
		white-space: nowrap; flex-shrink: 0;
	}
	.sub-card__status--active { background-color: rgba(34, 181, 115, 0.12); color: var(--color-green); }
	.sub-card__status--canceled { background-color: rgba(255, 255, 255, 0.06); color: var(--color-grey-400); }
	.sub-card__status--past_due { background-color: rgba(224, 72, 72, 0.12); color: var(--color-red); }
	.sub-card__details { display: flex; flex-direction: column; gap: 0.35rem; }
	.sub-card__row { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
	.sub-card__label { font-size: var(--fs-xs); color: var(--color-grey-400); }
	.sub-card__value { font-size: var(--fs-sm); color: var(--color-grey-300); text-align: right; }
	.sub-card__plan-badge {
		font-size: var(--fs-xs); font-weight: var(--w-semibold);
		padding: 0.15rem 0.5rem; border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.12); color: var(--color-teal);
	}

	/* Pagination */
	.subs-page__pagination { display: flex; align-items: center; justify-content: center; gap: 0.75rem; margin-top: 1rem; }
	.subs-page__page-btn {
		display: flex; align-items: center; gap: 0.25rem;
		padding: 0.5rem 0.75rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white); font-size: var(--fs-xs);
		cursor: pointer; transition: border-color var(--duration-200) var(--ease-out);
	}
	.subs-page__page-btn:hover:not(:disabled) { border-color: var(--color-teal); }
	.subs-page__page-btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.subs-page__page-info { font-size: var(--fs-xs); color: var(--color-grey-400); }

	/* Tablet 480px+ */
	@media (min-width: 480px) {
		.subs-page__kpis { grid-template-columns: repeat(2, 1fr); }
		.subs-page__filters { flex-direction: row; }
		.subs-page__search { flex: 1; }
		.subs-page__filter-group { flex: 0 0 auto; }
		.subs-page__select { min-width: 10rem; }
	}

	/* Tablet+ 768px+ */
	@media (min-width: 768px) {
		.subs-page__header { flex-direction: row; justify-content: space-between; align-items: flex-start; margin-bottom: 1.5rem; }
		.subs-page__title { font-size: var(--fs-2xl); }
		.subs-page__count { font-size: var(--fs-sm); margin-top: 0.25rem; }
		.subs-page__plans-link { align-self: auto; padding: 0.6rem 1.25rem; font-size: var(--fs-sm); }
		.subs-page__kpis { grid-template-columns: repeat(4, 1fr); gap: 1rem; margin-bottom: 2rem; }
		.kpi { padding: 1.15rem; gap: 1rem; }
		.kpi__icon { width: 2.75rem; height: 2.75rem; }
		.kpi__value { font-size: var(--fs-lg); }
		.subs-page__filters { margin-bottom: 1.5rem; }
		.subs-page__cards { display: none; }
		.subs-page__table-wrap {
			display: block; overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
		}
		.s-table { width: 100%; border-collapse: collapse; }
		.s-table th {
			text-align: left; font-size: var(--fs-xs); font-weight: var(--w-semibold);
			color: var(--color-grey-400); text-transform: uppercase; letter-spacing: 0.05em;
			padding: 0.85rem 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}
		.s-table td {
			padding: 0.85rem 1rem; font-size: var(--fs-sm);
			color: var(--color-grey-300); border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		}
		.s-table__row--clickable { cursor: pointer; transition: background-color var(--duration-200) var(--ease-out); }
		.s-table__row--clickable:hover { background-color: rgba(255, 255, 255, 0.02); }
		.s-table__member { display: flex; flex-direction: column; gap: 0.1rem; }
		.s-table__name { font-weight: var(--w-semibold); color: var(--color-white); }
		.s-table__email { font-size: var(--fs-xs); color: var(--color-grey-500); }
		.s-table__plan-badge {
			font-size: var(--fs-xs); font-weight: var(--w-semibold);
			padding: 0.15rem 0.5rem; border-radius: var(--radius-full);
			background-color: rgba(15, 164, 175, 0.12); color: var(--color-teal);
		}
		.s-table__status {
			display: inline-block; font-size: var(--fs-xs); font-weight: var(--w-semibold);
			padding: 0.15rem 0.55rem; border-radius: var(--radius-full);
		}
		.s-table__status--active { background-color: rgba(34, 181, 115, 0.12); color: var(--color-green); }
		.s-table__status--canceled { background-color: rgba(255, 255, 255, 0.06); color: var(--color-grey-400); }
		.s-table__status--past_due { background-color: rgba(224, 72, 72, 0.12); color: var(--color-red); }
		.s-table__amount { font-weight: var(--w-semibold); color: var(--color-white); }
		.s-table__interval { font-weight: var(--w-regular); color: var(--color-grey-500); font-size: var(--fs-xs); }
		.subs-page__pagination { gap: 1rem; margin-top: 1.5rem; }
		.subs-page__page-btn { gap: 0.35rem; padding: 0.5rem 1rem; font-size: var(--fs-sm); }
		.subs-page__page-info { font-size: var(--fs-sm); }
	}

	/* Desktop 1024px+ */
	@media (min-width: 1024px) {
		.subs-page__kpis { grid-template-columns: repeat(4, 1fr); }
		.kpi { flex-direction: column; text-align: center; gap: 0.5rem; }
		.kpi__icon { width: 3rem; height: 3rem; }
	}
</style>
