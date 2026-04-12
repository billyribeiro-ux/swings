<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { Coupon, BulkCouponPayload, PaginatedResponse } from '$lib/api/types';
	import CaretLeft from 'phosphor-svelte/lib/CaretLeft';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import Ticket from 'phosphor-svelte/lib/Ticket';
	import ChartBar from 'phosphor-svelte/lib/ChartBar';
	import CurrencyDollar from 'phosphor-svelte/lib/CurrencyDollar';
	import Plus from 'phosphor-svelte/lib/Plus';
	import ToggleLeft from 'phosphor-svelte/lib/ToggleLeft';
	import ToggleRight from 'phosphor-svelte/lib/ToggleRight';
	import Lightning from 'phosphor-svelte/lib/Lightning';

	interface CouponStats {
		active_count: number;
		total_usages: number;
		total_discount_cents: number;
	}

	type FilterTab = 'all' | 'active' | 'expired' | 'inactive';

	let coupons = $state<Coupon[]>([]);
	let stats = $state<CouponStats | null>(null);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let statsLoading = $state(true);
	let search = $state('');
	let filter = $state<FilterTab>('all');
	let togglingId = $state<string | null>(null);
	let showBulkForm = $state(false);
	let bulkLoading = $state(false);
	let bulkCount = $state(10);
	let bulkPrefix = $state('');
	let bulkDiscountType = $state<'percentage' | 'fixed_amount'>('percentage');
	let bulkDiscountValue = $state(10);
	let bulkUsageLimit = $state<number | undefined>(undefined);
	let bulkExpiresAt = $state('');

	let searchTimeout: ReturnType<typeof setTimeout>;
	const filters: { label: string; value: FilterTab }[] = [
		{ label: 'All', value: 'all' },
		{ label: 'Active', value: 'active' },
		{ label: 'Expired', value: 'expired' },
		{ label: 'Inactive', value: 'inactive' }
	];

	function handleSearchInput(value: string) {
		search = value;
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => { page = 1; loadCoupons(); }, 300);
	}

	function setFilter(tab: FilterTab) {
		filter = tab;
		page = 1;
		loadCoupons();
	}

	async function loadStats() {
		statsLoading = true;
		try {
			stats = await api.get<CouponStats>('/api/admin/coupons/stats');
		} catch { /* silent */ } finally { statsLoading = false; }
	}

	async function loadCoupons() {
		loading = true;
		try {
			const params = new URLSearchParams({ page: String(page), per_page: '15' });
			if (search.trim()) params.set('search', search.trim());
			if (filter !== 'all') params.set('status', filter);
			const res = await api.get<PaginatedResponse<Coupon>>(`/api/admin/coupons?${params.toString()}`);
			coupons = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch { /* silent */ } finally { loading = false; }
	}

	async function toggleActive(coupon: Coupon) {
		togglingId = coupon.id;
		try {
			await api.put(`/api/admin/coupons/${coupon.id}`, { is_active: !coupon.is_active });
			coupon.is_active = !coupon.is_active;
			await loadStats();
		} catch { alert('Failed to update coupon'); } finally { togglingId = null; }
	}

	async function submitBulk() {
		bulkLoading = true;
		try {
			const payload: BulkCouponPayload = {
				count: bulkCount,
				prefix: bulkPrefix || undefined,
				discount_type: bulkDiscountType,
				discount_value: bulkDiscountValue,
				usage_limit: bulkUsageLimit,
				expires_at: bulkExpiresAt || undefined
			};
			await api.post('/api/admin/coupons/bulk', payload);
			showBulkForm = false;
			bulkCount = 10; bulkPrefix = ''; bulkDiscountValue = 10; bulkUsageLimit = undefined; bulkExpiresAt = '';
			await Promise.all([loadCoupons(), loadStats()]);
		} catch { alert('Failed to generate coupons'); } finally { bulkLoading = false; }
	}

	onMount(() => { loadStats(); loadCoupons(); });

	function formatDiscount(coupon: Coupon): string {
		if (coupon.discount_type === 'percentage') return `${coupon.discount_value}% off`;
		if (coupon.discount_type === 'fixed_amount') return `$${(coupon.discount_value / 100).toFixed(0)} off`;
		return `${coupon.discount_value}-day trial`;
	}

	function formatUsage(coupon: Coupon): string {
		return coupon.usage_limit ? `${coupon.usage_count}/${coupon.usage_limit}` : `${coupon.usage_count}`;
	}

	function couponStatus(coupon: Coupon): 'active' | 'expired' | 'inactive' {
		if (!coupon.is_active) return 'inactive';
		if (coupon.expires_at && new Date(coupon.expires_at) < new Date()) return 'expired';
		return 'active';
	}

	function statusLabel(status: string): string {
		return status.charAt(0).toUpperCase() + status.slice(1);
	}

	function formatMoney(cents: number): string {
		return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	}
</script>

<svelte:head>
	<title>Coupons - Admin - Explosive Swings</title>
</svelte:head>

<div class="cp-page">
	<!-- Header -->
	<div class="cp-page__header">
		<div>
			<h1 class="cp-page__title">Coupons</h1>
			<p class="cp-page__count">{total} total coupons</p>
		</div>
		<div class="cp-page__actions">
			<button class="cp-page__bulk-btn" onclick={() => (showBulkForm = !showBulkForm)}>
				<Lightning size={16} weight="bold" /> Bulk Generate
			</button>
			<a href="/admin/coupons/new" class="cp-page__create-link">
				<Plus size={16} weight="bold" /> Create Coupon
			</a>
		</div>
	</div>

	<!-- Bulk Generate Inline Form -->
	{#if showBulkForm}
		<form class="bulk-form" onsubmit={(e) => { e.preventDefault(); submitBulk(); }}>
			<h3 class="bulk-form__title">Bulk Generate Coupons</h3>
			<div class="bulk-form__grid">
				<label class="bulk-form__field">
					<span class="bulk-form__label">Count</span>
					<input type="number" min="1" max="500" bind:value={bulkCount} class="bulk-form__input" required />
				</label>
				<label class="bulk-form__field">
					<span class="bulk-form__label">Code Prefix</span>
					<input type="text" placeholder="e.g. SUMMER" bind:value={bulkPrefix} class="bulk-form__input" />
				</label>
				<label class="bulk-form__field">
					<span class="bulk-form__label">Discount Type</span>
					<select bind:value={bulkDiscountType} class="bulk-form__input">
						<option value="percentage">Percentage</option>
						<option value="fixed_amount">Fixed Amount</option>
					</select>
				</label>
				<label class="bulk-form__field">
					<span class="bulk-form__label">Value {bulkDiscountType === 'percentage' ? '(%)' : '(cents)'}</span>
					<input type="number" min="1" bind:value={bulkDiscountValue} class="bulk-form__input" required />
				</label>
				<label class="bulk-form__field">
					<span class="bulk-form__label">Usage Limit</span>
					<input type="number" min="1" placeholder="Unlimited" bind:value={bulkUsageLimit} class="bulk-form__input" />
				</label>
				<label class="bulk-form__field">
					<span class="bulk-form__label">Expires At</span>
					<input type="date" bind:value={bulkExpiresAt} class="bulk-form__input" />
				</label>
			</div>
			<div class="bulk-form__footer">
				<button type="button" class="bulk-form__cancel" onclick={() => (showBulkForm = false)}>Cancel</button>
				<button type="submit" class="bulk-form__submit" disabled={bulkLoading}>
					{bulkLoading ? 'Generating...' : `Generate ${bulkCount} Coupons`}
				</button>
			</div>
		</form>
	{/if}

	<!-- Stats Row -->
	{#if statsLoading}
		<div class="cp-page__kpis">
			{#each Array(3) as _, i (i)}
				<div class="kpi kpi--skeleton"><div class="kpi__icon-skeleton"></div><div class="kpi__text-skeleton"><div class="skeleton-line skeleton-line--short"></div><div class="skeleton-line skeleton-line--long"></div></div></div>
			{/each}
		</div>
	{:else if stats}
		<div class="cp-page__kpis">
			<div class="kpi">
				<div class="kpi__icon kpi__icon--green"><Ticket size={22} weight="fill" /></div>
				<div><p class="kpi__label">Active Coupons</p><p class="kpi__value">{stats.active_count.toLocaleString()}</p></div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--blue"><ChartBar size={22} weight="fill" /></div>
				<div><p class="kpi__label">Total Usages</p><p class="kpi__value">{stats.total_usages.toLocaleString()}</p></div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--teal"><CurrencyDollar size={22} weight="fill" /></div>
				<div><p class="kpi__label">Total Discount</p><p class="kpi__value">{formatMoney(stats.total_discount_cents)}</p></div>
			</div>
		</div>
	{/if}

	<!-- Filters + Search -->
	<div class="cp-page__toolbar">
		<div class="cp-page__tabs">
			{#each filters as tab (tab.value)}
				<button
					class="cp-page__tab"
					class:cp-page__tab--active={filter === tab.value}
					onclick={() => setFilter(tab.value)}
				>{tab.label}</button>
			{/each}
		</div>
		<div class="cp-page__search">
			<MagnifyingGlass size={18} weight="bold" />
			<input type="text" placeholder="Search by code..." value={search} oninput={(e) => handleSearchInput(e.currentTarget.value)} class="cp-page__search-input" />
		</div>
	</div>

	<!-- Content -->
	{#if loading}
		<div class="cp-page__skeleton-list">
			{#each Array(6) as _, i (i)}
				<div class="skeleton-row"><div class="skeleton-line skeleton-line--full"></div></div>
			{/each}
		</div>
	{:else if coupons.length === 0}
		<div class="cp-page__empty"><p>No coupons found.</p></div>
	{:else}
		<!-- Mobile Cards -->
		<div class="cp-page__cards">
			{#each coupons as coupon (coupon.id)}
				{@const status = couponStatus(coupon)}
				<div class="coupon-card">
					<div class="coupon-card__header">
						<a href="/admin/coupons/{coupon.id}" class="coupon-card__code">{coupon.code}</a>
						<span class="badge badge--{status}">{statusLabel(status)}</span>
					</div>
					<div class="coupon-card__row"><span class="coupon-card__label">Discount</span><span class="coupon-card__value">{formatDiscount(coupon)}</span></div>
					<div class="coupon-card__row"><span class="coupon-card__label">Usage</span><span class="coupon-card__value">{formatUsage(coupon)}</span></div>
					<div class="coupon-card__footer">
						<button class="toggle-btn" onclick={() => toggleActive(coupon)} disabled={togglingId === coupon.id} title={coupon.is_active ? 'Deactivate' : 'Activate'}>
							{#if coupon.is_active}<ToggleRight size={24} weight="fill" />{:else}<ToggleLeft size={24} weight="bold" />{/if}
						</button>
						<a href="/admin/coupons/{coupon.id}" class="coupon-card__edit">Edit</a>
					</div>
				</div>
			{/each}
		</div>

		<!-- Table -->
		<div class="cp-page__table-wrap">
			<table class="c-table">
				<thead>
					<tr>
						<th>Code</th>
						<th>Discount</th>
						<th>Usage</th>
						<th>Status</th>
						<th>Active</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each coupons as coupon (coupon.id)}
						{@const status = couponStatus(coupon)}
						<tr>
							<td><a href="/admin/coupons/{coupon.id}" class="c-table__code">{coupon.code}</a></td>
							<td>{formatDiscount(coupon)}</td>
							<td>{formatUsage(coupon)}</td>
							<td><span class="badge badge--{status}">{statusLabel(status)}</span></td>
							<td>
								<button class="toggle-btn" onclick={() => toggleActive(coupon)} disabled={togglingId === coupon.id} title={coupon.is_active ? 'Deactivate' : 'Activate'}>
									{#if coupon.is_active}<ToggleRight size={22} weight="fill" />{:else}<ToggleLeft size={22} weight="bold" />{/if}
								</button>
							</td>
							<td><a href="/admin/coupons/{coupon.id}" class="c-table__link">Edit</a></td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	<!-- Pagination -->
	{#if totalPages > 1}
		<div class="cp-page__pagination">
			<button onclick={() => { page--; loadCoupons(); }} disabled={page <= 1} class="cp-page__page-btn">
				<CaretLeft size={16} weight="bold" /> Prev
			</button>
			<span class="cp-page__page-info">Page {page} of {totalPages}</span>
			<button onclick={() => { page++; loadCoupons(); }} disabled={page >= totalPages} class="cp-page__page-btn">
				Next <CaretRight size={16} weight="bold" />
			</button>
		</div>
	{/if}
</div>

<style>
	/* ========= BASE / MOBILE ========= */
	.cp-page__header { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.25rem; }
	.cp-page__title { font-size: var(--fs-xl); font-weight: var(--w-bold); color: var(--color-white); font-family: var(--font-heading); }
	.cp-page__count { font-size: var(--fs-xs); color: var(--color-grey-400); margin-top: 0.15rem; }
	.cp-page__actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }
	.cp-page__create-link {
		display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.55rem 1rem;
		font-size: var(--fs-xs); font-weight: var(--w-semibold); color: var(--color-white);
		background: linear-gradient(135deg, var(--color-teal), #0d8a94); border-radius: var(--radius-lg);
		text-decoration: none; transition: opacity 200ms var(--ease-out), transform 200ms var(--ease-out);
	}
	.cp-page__create-link:hover { opacity: 0.9; transform: translateY(-1px); }
	.cp-page__bulk-btn {
		display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.55rem 1rem;
		font-size: var(--fs-xs); font-weight: var(--w-semibold); color: var(--color-gold);
		background: rgba(212, 168, 67, 0.1); border: 1px solid rgba(212, 168, 67, 0.25);
		border-radius: var(--radius-lg); cursor: pointer;
		transition: background-color 200ms var(--ease-out), border-color 200ms var(--ease-out);
	}
	.cp-page__bulk-btn:hover { background: rgba(212, 168, 67, 0.2); border-color: rgba(212, 168, 67, 0.4); }

	/* Bulk Form */
	.bulk-form {
		padding: 1rem; margin-bottom: 1.25rem; background-color: var(--color-navy-mid);
		border: 1px solid rgba(212, 168, 67, 0.2); border-radius: var(--radius-xl);
	}
	.bulk-form__title { font-size: var(--fs-sm); font-weight: var(--w-semibold); color: var(--color-gold); margin-bottom: 0.75rem; }
	.bulk-form__grid { display: grid; grid-template-columns: 1fr; gap: 0.75rem; }
	.bulk-form__field { display: flex; flex-direction: column; gap: 0.25rem; }
	.bulk-form__label { font-size: var(--fs-xs); color: var(--color-grey-400); }
	.bulk-form__input {
		padding: 0.55rem 0.75rem; background-color: var(--color-navy); border: 1px solid rgba(255,255,255,0.1);
		border-radius: var(--radius-md); color: var(--color-white); font-size: var(--fs-sm); font-family: var(--font-ui);
	}
	.bulk-form__input:focus { outline: none; border-color: var(--color-teal); }
	.bulk-form__footer { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem; }
	.bulk-form__cancel {
		padding: 0.5rem 1rem; font-size: var(--fs-xs); color: var(--color-grey-400); background: none;
		border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-lg); cursor: pointer;
	}
	.bulk-form__cancel:hover { border-color: rgba(255,255,255,0.2); color: var(--color-white); }
	.bulk-form__submit {
		padding: 0.5rem 1.25rem; font-size: var(--fs-xs); font-weight: var(--w-semibold);
		color: var(--color-white); background: linear-gradient(135deg, var(--color-gold), #b8912e);
		border: none; border-radius: var(--radius-lg); cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.bulk-form__submit:hover { opacity: 0.9; }
	.bulk-form__submit:disabled { opacity: 0.5; cursor: not-allowed; }

	/* KPIs */
	.cp-page__kpis { display: grid; grid-template-columns: 1fr; gap: 0.75rem; margin-bottom: 1.25rem; }
	.kpi { display: flex; align-items: center; gap: 0.75rem; padding: 1rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-xl); }
	.kpi--skeleton { min-height: 4rem; }
	.kpi__icon-skeleton { width: 2.5rem; height: 2.5rem; border-radius: var(--radius-lg); background: rgba(255,255,255,0.06); animation: shimmer 1.5s infinite; flex-shrink: 0; }
	.kpi__text-skeleton { display: flex; flex-direction: column; gap: 0.4rem; flex: 1; }
	.skeleton-line { height: 0.75rem; border-radius: var(--radius-sm); background: rgba(255,255,255,0.06); animation: shimmer 1.5s infinite; }
	.skeleton-line--short { width: 40%; }
	.skeleton-line--long { width: 65%; height: 1rem; }
	.skeleton-line--full { width: 100%; height: 2.5rem; }
	@keyframes shimmer { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.8; } }
	.kpi__icon { width: 2.5rem; height: 2.5rem; border-radius: var(--radius-lg); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
	.kpi__icon--green { background-color: rgba(34, 197, 94, 0.15); color: #22c55e; }
	.kpi__icon--blue { background-color: rgba(59, 130, 246, 0.15); color: #3b82f6; }
	.kpi__icon--teal { background-color: rgba(15, 164, 175, 0.15); color: var(--color-teal); }
	.kpi__label { font-size: var(--fs-xs); color: var(--color-grey-400); margin-bottom: 0.1rem; }
	.kpi__value { font-size: var(--fs-md); font-weight: var(--w-bold); color: var(--color-white); }

	/* Toolbar */
	.cp-page__toolbar { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.25rem; }
	.cp-page__tabs { display: flex; gap: 0.25rem; overflow-x: auto; }
	.cp-page__tab {
		padding: 0.5rem 0.85rem; font-size: var(--fs-xs); font-weight: var(--w-medium);
		color: var(--color-grey-400); background: none; border: 1px solid transparent;
		border-radius: var(--radius-lg); cursor: pointer; white-space: nowrap;
		transition: color 200ms var(--ease-out), background-color 200ms var(--ease-out), border-color 200ms var(--ease-out);
	}
	.cp-page__tab:hover { color: var(--color-white); background-color: rgba(255,255,255,0.04); }
	.cp-page__tab--active { color: var(--color-teal-light); background-color: var(--color-teal-glow); border-color: rgba(15,164,175,0.3); }
	.cp-page__search {
		display: flex; align-items: center; gap: 0.5rem; padding: 0.65rem 0.85rem;
		background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.1);
		border-radius: var(--radius-lg); color: var(--color-grey-400); transition: border-color 200ms var(--ease-out);
	}
	.cp-page__search:focus-within { border-color: var(--color-teal); }
	.cp-page__search-input { flex: 1; background: none; border: none; outline: none; color: var(--color-white); font-size: var(--fs-sm); font-family: var(--font-ui); }
	.cp-page__search-input::placeholder { color: var(--color-grey-500); }

	/* Skeleton / Empty */
	.cp-page__skeleton-list { display: flex; flex-direction: column; gap: 0.5rem; }
	.skeleton-row { padding: 1rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-lg); }
	.cp-page__empty { text-align: center; padding: 3rem 1rem; color: var(--color-grey-400); font-size: var(--fs-sm); background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-xl); }

	/* Badge */
	.badge { display: inline-block; font-size: var(--fs-xs); font-weight: var(--w-semibold); padding: 0.15rem 0.55rem; border-radius: var(--radius-full); }
	.badge--active { background-color: rgba(34, 181, 115, 0.12); color: var(--color-green); }
	.badge--expired { background-color: rgba(224, 72, 72, 0.12); color: var(--color-red); }
	.badge--inactive { background-color: rgba(255, 255, 255, 0.06); color: var(--color-grey-400); }

	/* Toggle Button */
	.toggle-btn { background: none; border: none; cursor: pointer; padding: 0.15rem; display: flex; align-items: center; transition: opacity 200ms var(--ease-out); }
	.toggle-btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.toggle-btn :global(svg) { color: var(--color-teal); }

	/* Mobile Cards */
	.cp-page__cards { display: flex; flex-direction: column; gap: 0.5rem; }
	.cp-page__table-wrap { display: none; }
	.coupon-card { display: flex; flex-direction: column; gap: 0.5rem; padding: 0.85rem 1rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-lg); }
	.coupon-card__header { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
	.coupon-card__code { font-family: 'Courier New', Courier, monospace; font-weight: var(--w-semibold); color: var(--color-white); text-decoration: none; font-size: var(--fs-sm); letter-spacing: 0.04em; }
	.coupon-card__code:hover { color: var(--color-teal-light); }
	.coupon-card__row { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
	.coupon-card__label { font-size: var(--fs-xs); color: var(--color-grey-400); }
	.coupon-card__value { font-size: var(--fs-sm); color: var(--color-grey-300); }
	.coupon-card__footer { display: flex; justify-content: space-between; align-items: center; padding-top: 0.5rem; border-top: 1px solid rgba(255,255,255,0.06); }
	.coupon-card__edit { font-size: var(--fs-sm); font-weight: var(--w-semibold); color: var(--color-teal-light); text-decoration: none; }
	.coupon-card__edit:hover { text-decoration: underline; }

	/* Pagination */
	.cp-page__pagination { display: flex; align-items: center; justify-content: center; gap: 0.75rem; margin-top: 1rem; }
	.cp-page__page-btn {
		display: flex; align-items: center; gap: 0.25rem; padding: 0.5rem 0.75rem;
		background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.1);
		border-radius: var(--radius-lg); color: var(--color-white); font-size: var(--fs-xs);
		cursor: pointer; transition: border-color 200ms var(--ease-out);
	}
	.cp-page__page-btn:hover:not(:disabled) { border-color: var(--color-teal); }
	.cp-page__page-btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.cp-page__page-info { font-size: var(--fs-xs); color: var(--color-grey-400); }

	/* ========= TABLET (480px+) ========= */
	@media (min-width: 480px) {
		.cp-page__kpis { grid-template-columns: repeat(3, 1fr); }
		.bulk-form__grid { grid-template-columns: 1fr 1fr; }
	}

	/* ========= TABLET+ (768px+) ========= */
	@media (min-width: 768px) {
		.cp-page__header { flex-direction: row; justify-content: space-between; align-items: flex-start; margin-bottom: 1.5rem; }
		.cp-page__title { font-size: var(--fs-2xl); }
		.cp-page__count { font-size: var(--fs-sm); margin-top: 0.25rem; }
		.cp-page__create-link, .cp-page__bulk-btn { padding: 0.6rem 1.25rem; font-size: var(--fs-sm); }
		.cp-page__kpis { gap: 1rem; margin-bottom: 2rem; }
		.kpi { padding: 1.15rem; gap: 1rem; }
		.kpi__icon { width: 2.75rem; height: 2.75rem; }
		.kpi__value { font-size: var(--fs-lg); }
		.cp-page__toolbar { flex-direction: row; justify-content: space-between; align-items: center; margin-bottom: 1.5rem; }
		.cp-page__search { min-width: 14rem; }
		.bulk-form { padding: 1.25rem; }
		.bulk-form__grid { grid-template-columns: repeat(3, 1fr); }
		.bulk-form__title { font-size: var(--fs-md); }

		/* Hide cards, show table */
		.cp-page__cards { display: none; }
		.cp-page__table-wrap {
			display: block; overflow-x: auto; background-color: var(--color-navy-mid);
			border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-xl);
		}
		.c-table { width: 100%; border-collapse: collapse; }
		.c-table th {
			text-align: left; font-size: var(--fs-xs); font-weight: var(--w-semibold);
			color: var(--color-grey-400); text-transform: uppercase; letter-spacing: 0.05em;
			padding: 0.85rem 1rem; border-bottom: 1px solid rgba(255,255,255,0.06);
		}
		.c-table td { padding: 0.85rem 1rem; font-size: var(--fs-sm); color: var(--color-grey-300); border-bottom: 1px solid rgba(255,255,255,0.04); }
		.c-table tbody tr:hover { background-color: rgba(255,255,255,0.02); }
		.c-table__code {
			font-family: 'Courier New', Courier, monospace; font-weight: var(--w-semibold);
			color: var(--color-white); text-decoration: none; letter-spacing: 0.04em;
		}
		.c-table__code:hover { color: var(--color-teal-light); }
		.c-table__link { color: var(--color-teal-light); font-weight: var(--w-semibold); text-decoration: none; }
		.c-table__link:hover { text-decoration: underline; }
		.cp-page__pagination { gap: 1rem; margin-top: 1.5rem; }
		.cp-page__page-btn { gap: 0.35rem; padding: 0.5rem 1rem; font-size: var(--fs-sm); }
		.cp-page__page-info { font-size: var(--fs-sm); }
	}
</style>
