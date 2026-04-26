<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { goto } from '$app/navigation';
	import { toast } from '$lib/stores/toast.svelte';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import CurrencyDollarIcon from 'phosphor-svelte/lib/CurrencyDollarIcon';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import CalendarCheckIcon from 'phosphor-svelte/lib/CalendarCheckIcon';
	import RepeatIcon from 'phosphor-svelte/lib/RepeatIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';

	type SubscriptionStatus = 'active' | 'canceled' | 'past_due' | 'trialing' | 'unpaid';

	interface Subscription {
		id: string;
		member_id: string;
		member_name: string;
		member_email: string;
		plan_name: string;
		status: SubscriptionStatus;
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
	let statusFilter = $state<'all' | SubscriptionStatus>('all');

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
		statusFilter = value as 'all' | SubscriptionStatus;
		page = 1;
		loadSubscriptions();
	}

	async function loadStats() {
		statsLoading = true;
		try {
			stats = await api.get<SubscriptionStats>('/api/admin/subscriptions/stats');
		} catch (e) {
			stats = null;
			toast.error('Failed to load subscription stats', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			statsLoading = false;
		}
	}

	async function loadSubscriptions() {
		loading = true;
		try {
			const params: Array<[string, string]> = [
				['page', String(page)],
				['per_page', '15']
			];
			if (search.trim()) params.push(['search', search.trim()]);
			if (statusFilter !== 'all') params.push(['status', statusFilter]);
			const query = params
				.map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
				.join('&');

			const res = await api.get<PaginatedSubscriptions>(`/api/admin/subscriptions?${query}`);
			subscriptions = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch (e) {
			subscriptions = [];
			toast.error('Failed to load subscriptions', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadStats();
		loadSubscriptions();
	});

	function formatMoney(cents: number): string {
		return (
			'$' +
			(cents / 100).toLocaleString('en-US', {
				minimumFractionDigits: 2,
				maximumFractionDigits: 2
			})
		);
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function statusLabel(status: string): string {
		if (status === 'past_due') return 'Past due';
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
	<header class="subs-page__header">
		<div class="subs-page__heading">
			<span class="subs-page__eyebrow">Commerce</span>
			<h1 class="subs-page__title">Subscriptions</h1>
			<p class="subs-page__subtitle">
				Recurring revenue at a glance: active customers, MRR, plan mix, and per-member status.
				{total} total.
			</p>
		</div>
		<a href="/admin/subscriptions/plans" class="btn btn--primary">
			<RepeatIcon size={16} weight="bold" />
			<span>Manage plans</span>
		</a>
	</header>

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
					<UsersIcon size={22} weight="duotone" />
				</div>
				<div class="kpi__content">
					<span class="kpi__label">Total active</span>
					<span class="kpi__value">{stats.total_active.toLocaleString()}</span>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--blue">
					<CalendarCheckIcon size={22} weight="duotone" />
				</div>
				<div class="kpi__content">
					<span class="kpi__label">Monthly</span>
					<span class="kpi__value">{stats.monthly_count.toLocaleString()}</span>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--purple">
					<RepeatIcon size={22} weight="duotone" />
				</div>
				<div class="kpi__content">
					<span class="kpi__label">Annual</span>
					<span class="kpi__value">{stats.annual_count.toLocaleString()}</span>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--teal">
					<CurrencyDollarIcon size={22} weight="duotone" />
				</div>
				<div class="kpi__content">
					<span class="kpi__label">MRR</span>
					<span class="kpi__value">{formatMoney(stats.mrr_cents)}</span>
				</div>
			</div>
		</div>
	{/if}

	<div class="filter-card">
		<div class="filter-card__field filter-card__field--search">
			<label class="filter-card__label" for="sub-search">Search</label>
			<div class="search-wrap">
				<MagnifyingGlassIcon size={16} weight="bold" class="search-icon" />
				<input
					id="sub-search"
					name="sub-search"
					type="search"
					class="filter-input filter-input--search"
					placeholder="Search by name or email…"
					value={search}
					oninput={(e) => handleSearchInput(e.currentTarget.value)}
				/>
			</div>
		</div>
		<div class="filter-card__field">
			<label class="filter-card__label" for="sub-status">Status</label>
			<div class="select-wrap">
				<select
					id="sub-status"
					name="sub-status"
					class="filter-input filter-input--select"
					value={statusFilter}
					onchange={(e) => handleFilterChange(e.currentTarget.value)}
				>
					<option value="all">All statuses</option>
					<option value="active">Active</option>
					<option value="trialing">Trialing</option>
					<option value="past_due">Past due</option>
					<option value="unpaid">Unpaid</option>
					<option value="canceled">Canceled</option>
				</select>
				<CaretDownIcon size={14} weight="bold" class="select-caret" />
			</div>
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
		<div class="empty-state">
			<UsersIcon size={48} weight="duotone" />
			<h2 class="empty-state__title">No subscriptions found</h2>
			<p class="empty-state__desc">
				Try adjusting your search or status filter to find subscriptions.
			</p>
		</div>
	{:else}
		<div class="subs-page__cards">
			{#each subscriptions as sub (sub.id)}
				<button
					type="button"
					class="sub-card"
					onclick={() => navigateToMember(sub.member_id)}
				>
					<div class="sub-card__header">
						<div class="sub-card__member">
							<span class="sub-card__name">{sub.member_name}</span>
							<span class="sub-card__email">{sub.member_email}</span>
						</div>
						<span class="badge badge--{sub.status}">{statusLabel(sub.status)}</span>
					</div>
					<div class="sub-card__details">
						<div class="sub-card__row">
							<span class="sub-card__label">Plan</span>
							<span class="plan-chip">{sub.plan_name}</span>
						</div>
						<div class="sub-card__row">
							<span class="sub-card__label">Amount</span>
							<span class="sub-card__value sub-card__value--num">
								{formatMoney(sub.amount_cents)}<span class="sub-card__interval">
									/{sub.interval === 'month' ? 'mo' : 'yr'}
								</span>
							</span>
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
						<th class="s-table__num">Amount</th>
						<th>Started</th>
						<th>Next renewal</th>
						<th class="s-table__actions-h" aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each subscriptions as sub (sub.id)}
						<tr>
							<td>
								<div class="s-table__member">
									<span class="s-table__name">{sub.member_name}</span>
									<span class="s-table__email">{sub.member_email}</span>
								</div>
							</td>
							<td><span class="plan-chip">{sub.plan_name}</span></td>
							<td>
								<span class="badge badge--{sub.status}">{statusLabel(sub.status)}</span>
							</td>
							<td class="s-table__num">
								{formatMoney(sub.amount_cents)}<span class="s-table__interval">
									/{sub.interval === 'month' ? 'mo' : 'yr'}
								</span>
							</td>
							<td class="s-table__date">{formatDate(sub.start_date)}</td>
							<td class="s-table__date">
								{sub.next_renewal ? formatDate(sub.next_renewal) : '—'}
							</td>
							<td class="s-table__actions">
								<Tooltip label="View member {sub.member_name}">
									<button
										type="button"
										class="icon-btn"
										onclick={() => navigateToMember(sub.member_id)}
										aria-label="View member {sub.member_name}"
									>
										<EyeIcon size={16} weight="bold" />
									</button>
								</Tooltip>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if totalPages > 1}
		<nav class="subs-page__pagination" aria-label="Pagination">
			<button
				type="button"
				class="page-btn"
				onclick={() => {
					page--;
					loadSubscriptions();
				}}
				disabled={page <= 1}
			>
				<CaretLeftIcon size={14} weight="bold" />
				<span>Previous</span>
			</button>
			<span class="page-info">Page {page} of {totalPages}</span>
			<button
				type="button"
				class="page-btn"
				onclick={() => {
					page++;
					loadSubscriptions();
				}}
				disabled={page >= totalPages}
			>
				<span>Next</span>
				<CaretRightIcon size={14} weight="bold" />
			</button>
		</nav>
	{/if}
</div>

<style>
	.subs-page {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ── Header ─────────────────────────── */
	.subs-page__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: flex-start;
	}

	.subs-page__heading {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}

	.subs-page__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.subs-page__title {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		font-family: var(--font-heading);
		line-height: 1.2;
		letter-spacing: -0.01em;
		margin: 0;
	}

	.subs-page__subtitle {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
	}

	/* ── Buttons ────────────────────────── */
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0.55rem 1rem;
		font-size: 0.875rem;
		font-weight: 600;
		font-family: var(--font-ui);
		border-radius: var(--radius-lg);
		border: 1px solid transparent;
		text-decoration: none;
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			transform 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.btn--primary {
		color: var(--color-white);
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
	}

	.btn--primary:hover {
		transform: translateY(-1px);
		box-shadow: 0 10px 22px -4px rgba(15, 164, 175, 0.55);
	}

	/* ── KPIs ───────────────────────────── */
	.subs-page__kpis {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.75rem;
	}

	.kpi {
		display: flex;
		align-items: center;
		gap: 0.85rem;
		padding: 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.kpi--skeleton {
		min-height: 4.25rem;
	}

	.kpi__icon-skeleton {
		width: 2.75rem;
		height: 2.75rem;
		border-radius: var(--radius-lg);
		background: rgba(255, 255, 255, 0.06);
		animation: shimmer 1.5s infinite;
		flex-shrink: 0;
	}

	.kpi__text-skeleton {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		flex: 1;
	}

	.skeleton-line {
		height: 0.75rem;
		border-radius: var(--radius-sm);
		background: rgba(255, 255, 255, 0.06);
		animation: shimmer 1.5s infinite;
	}

	.skeleton-line--short {
		width: 40%;
	}

	.skeleton-line--long {
		width: 65%;
		height: 1rem;
	}

	.skeleton-line--full {
		width: 100%;
		height: 2.5rem;
	}

	@keyframes shimmer {
		0%,
		100% {
			opacity: 0.4;
		}
		50% {
			opacity: 0.8;
		}
	}

	.kpi__icon {
		width: 2.75rem;
		height: 2.75rem;
		border-radius: var(--radius-lg);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.kpi__icon--green {
		background-color: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}

	.kpi__icon--blue {
		background-color: rgba(59, 130, 246, 0.15);
		color: #60a5fa;
	}

	.kpi__icon--purple {
		background-color: rgba(168, 85, 247, 0.15);
		color: #c084fc;
	}

	.kpi__icon--teal {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	.kpi__content {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}

	.kpi__label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.kpi__value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.15;
		font-variant-numeric: tabular-nums;
	}

	/* ── Filter card ────────────────────── */
	.filter-card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.filter-card__field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		min-width: 0;
	}

	.filter-card__label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.search-wrap,
	.select-wrap {
		position: relative;
	}

	:global(.search-icon) {
		position: absolute;
		left: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	:global(.select-caret) {
		position: absolute;
		right: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	.filter-input {
		width: 100%;
		min-height: 2.5rem;
		padding: 0.65rem 0.875rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.875rem;
		font-family: var(--font-ui);
		outline: none;
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.filter-input::placeholder {
		color: var(--color-grey-500);
	}

	.filter-input:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.filter-input--search {
		padding-left: 2.25rem;
	}

	.filter-input--select {
		appearance: none;
		-webkit-appearance: none;
		-moz-appearance: none;
		padding-right: 2.25rem;
		cursor: pointer;
	}

	.filter-input--select option {
		background-color: var(--color-navy-mid);
		color: var(--color-white);
	}

	/* ── Skeleton/Empty ─────────────────── */
	.subs-page__skeleton-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.skeleton-row {
		padding: 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		gap: 0.85rem;
		padding: 3.5rem 2rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		color: var(--color-grey-500);
	}

	.empty-state :global(svg) {
		color: var(--color-grey-500);
	}

	.empty-state__title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0;
	}

	.empty-state__desc {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 36ch;
		line-height: 1.55;
		margin: 0;
	}

	/* ── Badges ─────────────────────────── */
	.badge {
		display: inline-flex;
		align-items: center;
		font-size: 0.6875rem;
		font-weight: 600;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.badge--active {
		background-color: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}

	.badge--trialing {
		background-color: rgba(168, 85, 247, 0.15);
		color: #c084fc;
	}

	.badge--canceled {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	.badge--past_due {
		background-color: rgba(245, 158, 11, 0.12);
		color: #fcd34d;
	}

	.badge--unpaid {
		background-color: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
	}

	/* ── Plan chip ──────────────────────── */
	.plan-chip {
		display: inline-flex;
		align-items: center;
		font-size: 0.75rem;
		font-weight: 600;
		padding: 0.15rem 0.55rem;
		border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.12);
		color: #5eead4;
		white-space: nowrap;
	}

	/* ── Mobile cards ───────────────────── */
	.subs-page__cards {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.subs-page__table-wrap {
		display: none;
	}

	.sub-card {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
		padding: 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		cursor: pointer;
		text-align: left;
		width: 100%;
		font-family: var(--font-ui);
		color: inherit;
		transition:
			border-color 200ms var(--ease-out),
			background-color 200ms var(--ease-out);
	}

	.sub-card:hover {
		border-color: rgba(15, 164, 175, 0.3);
		background-color: rgba(255, 255, 255, 0.03);
	}

	.sub-card__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.sub-card__member {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}

	.sub-card__name {
		font-weight: 600;
		color: var(--color-white);
		font-size: 0.875rem;
	}

	.sub-card__email {
		font-size: 0.75rem;
		color: var(--color-grey-400);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.sub-card__details {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.sub-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.sub-card__label {
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.sub-card__value {
		font-size: 0.875rem;
		color: var(--color-grey-200);
		text-align: right;
	}

	.sub-card__value--num {
		color: var(--color-white);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}

	.sub-card__interval {
		color: var(--color-grey-500);
		font-weight: var(--w-regular);
		font-size: 0.75rem;
	}

	/* ── Pagination ─────────────────────── */
	.subs-page__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 0.5rem;
	}

	.page-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		min-height: 2.25rem;
		padding: 0.45rem 0.75rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.875rem;
		font-weight: 600;
		font-family: var(--font-ui);
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out);
	}

	.page-btn:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.page-info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
	}

	/* ── Icon button (table) ────────────── */
	.icon-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		color: var(--color-grey-300);
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			color 150ms var(--ease-out);
	}

	.icon-btn:hover {
		background-color: rgba(15, 164, 175, 0.12);
		border-color: rgba(15, 164, 175, 0.3);
		color: var(--color-teal-light);
	}

	/* ── Tablet 480px+ ──────────────────── */
	@media (min-width: 480px) {
		.subs-page__kpis {
			grid-template-columns: repeat(2, 1fr);
		}

		.filter-card {
			flex-direction: row;
			align-items: flex-end;
		}

		.filter-card__field--search {
			flex: 1 1 18rem;
		}

		.filter-card__field {
			flex: 0 0 12rem;
		}
	}

	/* ── Tablet 768px+ ──────────────────── */
	@media (min-width: 768px) {
		.subs-page {
			gap: 1.5rem;
		}

		.subs-page__header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
		}


		.subs-page__kpis {
			grid-template-columns: repeat(4, 1fr);
			gap: 1rem;
		}

		.kpi {
			padding: 1.5rem;
		}


		.filter-card {
			padding: 1.5rem;
		}

		.subs-page__cards {
			display: none;
		}

		.subs-page__table-wrap {
			display: block;
			overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
			box-shadow:
				0 1px 0 rgba(255, 255, 255, 0.03) inset,
				0 12px 32px rgba(0, 0, 0, 0.18);
		}

		.s-table {
			width: 100%;
			border-collapse: collapse;
			min-width: 720px;
		}

		.s-table thead {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.s-table th {
			text-align: left;
			font-size: 0.6875rem;
			font-weight: 600;
			color: var(--color-grey-500);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			padding: 0.875rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
			white-space: nowrap;
		}

		.s-table td {
			padding: 0.875rem 1rem;
			font-size: 0.875rem;
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			line-height: 1.45;
		}

		.s-table tbody tr {
			transition: background-color 150ms var(--ease-out);
		}

		.s-table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.s-table tbody tr:last-child td {
			border-bottom: none;
		}

		.s-table__member {
			display: flex;
			flex-direction: column;
			gap: 0.1rem;
		}

		.s-table__name {
			font-weight: 600;
			color: var(--color-white);
		}

		.s-table__email {
			font-size: 0.75rem;
			color: var(--color-grey-500);
		}

		.s-table__num {
			text-align: right;
			font-variant-numeric: tabular-nums;
			font-weight: 500;
			color: var(--color-white);
			white-space: nowrap;
		}

		.s-table__interval {
			color: var(--color-grey-500);
			font-weight: var(--w-regular);
			font-size: 0.75rem;
		}

		.s-table__date {
			white-space: nowrap;
			font-size: 0.75rem;
			color: var(--color-grey-400);
			font-variant-numeric: tabular-nums;
		}

		.s-table__actions-h,
		.s-table__actions {
			width: 3rem;
			text-align: right;
		}

		.subs-page__pagination {
			gap: 1rem;
		}
	}
</style>
