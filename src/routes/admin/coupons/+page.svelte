<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { toast } from '$lib/stores/toast.svelte';
	import type { Coupon, BulkCouponPayload, PaginatedResponse } from '$lib/api/types';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import TicketIcon from 'phosphor-svelte/lib/TicketIcon';
	import ChartBarIcon from 'phosphor-svelte/lib/ChartBarIcon';
	import CurrencyDollarIcon from 'phosphor-svelte/lib/CurrencyDollarIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import ToggleLeftIcon from 'phosphor-svelte/lib/ToggleLeftIcon';
	import ToggleRightIcon from 'phosphor-svelte/lib/ToggleRightIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';

	interface CouponStats {
		active_count: number;
		total_usages: number;
		total_discount_cents: number;
	}
	type FilterTab = 'all' | 'active' | 'expired' | 'inactive';

	let coupons = $state<Coupon[]>([]);
	let stats = $state<CouponStats | null>(null);
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

	function handleSearchInput(value: string) {
		search = value;
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			page = 1;
			loadCoupons();
		}, 300);
	}

	function setFilter(value: string) {
		filter = value as FilterTab;
		page = 1;
		loadCoupons();
	}

	async function loadStats() {
		statsLoading = true;
		try {
			stats = await api.get<CouponStats>('/api/admin/coupons/stats');
		} catch (e) {
			toast.error('Failed to load coupon stats', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			statsLoading = false;
		}
	}

	async function loadCoupons() {
		loading = true;
		try {
			const params: Array<[string, string]> = [
				['page', String(page)],
				['per_page', '15']
			];
			if (search.trim()) params.push(['search', search.trim()]);
			if (filter !== 'all') params.push(['status', filter]);
			const query = params
				.map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
				.join('&');
			const res = await api.get<PaginatedResponse<Coupon>>(`/api/admin/coupons?${query}`);
			coupons = res.data;
			totalPages = res.total_pages;
		} catch (e) {
			toast.error('Failed to load coupons', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			loading = false;
		}
	}

	async function toggleActive(coupon: Coupon) {
		togglingId = coupon.id;
		const next = !coupon.is_active;
		try {
			await api.put(`/api/admin/coupons/${coupon.id}`, { is_active: next });
			coupon.is_active = next;
			toast.success(next ? `Activated ${coupon.code}` : `Deactivated ${coupon.code}`);
			await loadStats();
		} catch (e) {
			toast.error('Failed to update coupon', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			togglingId = null;
		}
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
			toast.success(`Generated ${bulkCount} coupon${bulkCount === 1 ? '' : 's'}`);
			showBulkForm = false;
			bulkCount = 10;
			bulkPrefix = '';
			bulkDiscountValue = 10;
			bulkUsageLimit = undefined;
			bulkExpiresAt = '';
			await Promise.all([loadCoupons(), loadStats()]);
		} catch (e) {
			toast.error('Failed to generate coupons', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			bulkLoading = false;
		}
	}

	onMount(() => {
		loadStats();
		loadCoupons();
	});

	function formatDiscount(c: Coupon): string {
		if (c.discount_type === 'percentage') return `${c.discount_value}% off`;
		if (c.discount_type === 'fixed_amount') return `$${(c.discount_value / 100).toFixed(0)} off`;
		return `${c.discount_value}-day trial`;
	}

	function formatUsage(c: Coupon): string {
		return c.usage_limit ? `${c.usage_count}/${c.usage_limit}` : `${c.usage_count}`;
	}

	function couponStatus(c: Coupon): 'active' | 'expired' | 'inactive' {
		if (!c.is_active) return 'inactive';
		if (c.expires_at && new Date(c.expires_at) < new Date()) return 'expired';
		return 'active';
	}

	function statusLabel(s: string): string {
		return s.charAt(0).toUpperCase() + s.slice(1);
	}

	function formatMoney(cents: number): string {
		return (
			'$' +
			(cents / 100).toLocaleString('en-US', {
				minimumFractionDigits: 2,
				maximumFractionDigits: 2
			})
		);
	}
</script>

<svelte:head>
	<title>Coupons - Admin - Precision Options Signals</title>
</svelte:head>

<div class="cp">
	<header class="cp__header">
		<div class="cp__heading">
			<span class="cp__eyebrow">Promotions</span>
			<h1 class="cp__title">Coupons</h1>
			<p class="cp__subtitle">
				Create and manage discount codes. Issue percent-off, fixed-amount, or trial coupons in bulk.
			</p>
		</div>
		<div class="cp__actions">
			<button type="button" class="btn btn--secondary" onclick={() => (showBulkForm = !showBulkForm)}>
				<LightningIcon size={16} weight="bold" />
				<span>Bulk generate</span>
			</button>
			<a href="/admin/coupons/new" class="btn btn--primary">
				<PlusIcon size={16} weight="bold" />
				<span>Create coupon</span>
			</a>
		</div>
	</header>

	{#if showBulkForm}
		<form
			class="bulk-form"
			onsubmit={(e) => {
				e.preventDefault();
				submitBulk();
			}}
		>
			<div class="bulk-form__head">
				<LightningIcon size={18} weight="duotone" />
				<h2 class="bulk-form__title">Bulk generate coupons</h2>
			</div>
			<div class="bulk-form__grid">
				<div class="bf-field">
					<label class="bf-label" for="bf-count">Count</label>
					<input
						id="bf-count"
						name="bf-count"
						type="number"
						min="1"
						max="500"
						bind:value={bulkCount}
						class="bf-input"
						required
					/>
				</div>
				<div class="bf-field">
					<label class="bf-label" for="bf-prefix">Code prefix</label>
					<input
						id="bf-prefix"
						name="bf-prefix"
						type="text"
						placeholder="e.g. SUMMER"
						bind:value={bulkPrefix}
						class="bf-input"
					/>
				</div>
				<div class="bf-field">
					<label class="bf-label" for="bf-type">Discount type</label>
					<div class="select-wrap">
						<select id="bf-type" name="bf-type" bind:value={bulkDiscountType} class="bf-input bf-input--select">
							<option value="percentage">Percentage</option>
							<option value="fixed_amount">Fixed amount</option>
						</select>
						<CaretDownIcon size={14} weight="bold" class="select-caret" />
					</div>
				</div>
				<div class="bf-field">
					<label class="bf-label" for="bf-value">
						Value {bulkDiscountType === 'percentage' ? '(%)' : '(cents)'}
					</label>
					<input
						id="bf-value"
						name="bf-value"
						type="number"
						min="1"
						bind:value={bulkDiscountValue}
						class="bf-input"
						required
					/>
				</div>
				<div class="bf-field">
					<label class="bf-label" for="bf-limit">Usage limit</label>
					<input
						id="bf-limit"
						name="bf-limit"
						type="number"
						min="1"
						placeholder="Unlimited"
						bind:value={bulkUsageLimit}
						class="bf-input"
					/>
				</div>
				<div class="bf-field">
					<label class="bf-label" for="bf-expires">Expires at</label>
					<input
						id="bf-expires"
						name="bf-expires"
						type="date"
						bind:value={bulkExpiresAt}
						class="bf-input"
					/>
				</div>
			</div>
			<div class="bulk-form__foot">
				<button type="button" class="btn btn--secondary" onclick={() => (showBulkForm = false)}>
					Cancel
				</button>
				<button type="submit" class="btn btn--primary" disabled={bulkLoading}>
					{bulkLoading ? 'Generating…' : `Generate ${bulkCount} coupons`}
				</button>
			</div>
		</form>
	{/if}

	{#if statsLoading}
		<div class="kpi-grid">
			{#each Array(3) as _, i (i)}
				<div class="kpi kpi--skeleton">
					<div class="kpi__icon-skel"></div>
					<div class="kpi__text-skel">
						<div class="skel skel--short"></div>
						<div class="skel skel--long"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if stats}
		<div class="kpi-grid">
			<div class="kpi">
				<div class="kpi__icon kpi__icon--green">
					<TicketIcon size={22} weight="duotone" />
				</div>
				<div class="kpi__content">
					<span class="kpi__label">Active coupons</span>
					<span class="kpi__value">{stats.active_count.toLocaleString()}</span>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--blue">
					<ChartBarIcon size={22} weight="duotone" />
				</div>
				<div class="kpi__content">
					<span class="kpi__label">Total usages</span>
					<span class="kpi__value">{stats.total_usages.toLocaleString()}</span>
				</div>
			</div>
			<div class="kpi">
				<div class="kpi__icon kpi__icon--teal">
					<CurrencyDollarIcon size={22} weight="duotone" />
				</div>
				<div class="kpi__content">
					<span class="kpi__label">Total discount</span>
					<span class="kpi__value">{formatMoney(stats.total_discount_cents)}</span>
				</div>
			</div>
		</div>
	{/if}

	<div class="filter-card">
		<div class="filter-card__field filter-card__field--search">
			<label class="filter-card__label" for="cp-search">Search</label>
			<div class="search-wrap">
				<MagnifyingGlassIcon size={16} weight="bold" class="search-icon" />
				<input
					id="cp-search"
					name="cp-search"
					type="search"
					class="filter-input filter-input--search"
					placeholder="Search by code…"
					value={search}
					oninput={(e) => handleSearchInput(e.currentTarget.value)}
				/>
			</div>
		</div>
		<div class="filter-card__field">
			<label class="filter-card__label" for="cp-status">Status</label>
			<div class="select-wrap">
				<select
					id="cp-status"
					name="cp-status"
					class="filter-input filter-input--select"
					value={filter}
					onchange={(e) => setFilter(e.currentTarget.value)}
				>
					<option value="all">All</option>
					<option value="active">Active</option>
					<option value="expired">Expired</option>
					<option value="inactive">Inactive</option>
				</select>
				<CaretDownIcon size={14} weight="bold" class="select-caret" />
			</div>
		</div>
	</div>

	{#if loading}
		<div class="cp__skel">
			{#each Array(6) as _, i (i)}
				<div class="skelrow">
					<div class="skel skel--full"></div>
				</div>
			{/each}
		</div>
	{:else if coupons.length === 0}
		<div class="empty-state">
			<TicketIcon size={48} weight="duotone" />
			<h2 class="empty-state__title">No coupons found</h2>
			<p class="empty-state__desc">
				Try adjusting your search or status filter, or create a new coupon to start offering discounts.
			</p>
		</div>
	{:else}
		<!-- Mobile: cards -->
		<div class="cp__cards">
			{#each coupons as coupon (coupon.id)}
				{@const st = couponStatus(coupon)}
				<div class="ccard">
					<div class="ccard__top">
						<a href="/admin/coupons/{coupon.id}" class="ccard__code">{coupon.code}</a>
						<span class="badge badge--{st}">{statusLabel(st)}</span>
					</div>
					<div class="ccard__row">
						<span class="ccard__label">Discount</span>
						<span class="ccard__value">{formatDiscount(coupon)}</span>
					</div>
					<div class="ccard__row">
						<span class="ccard__label">Usage</span>
						<span class="ccard__value">{formatUsage(coupon)}</span>
					</div>
					<div class="ccard__foot">
						<Tooltip label={coupon.is_active ? 'Deactivate coupon' : 'Activate coupon'}>
							<button
								type="button"
								class="toggle-btn"
								onclick={() => toggleActive(coupon)}
								disabled={togglingId === coupon.id}
								aria-label={coupon.is_active ? 'Deactivate coupon' : 'Activate coupon'}
							>
								{#if coupon.is_active}
									<ToggleRightIcon size={24} weight="fill" />
								{:else}
									<ToggleLeftIcon size={24} weight="bold" />
								{/if}
							</button>
						</Tooltip>
						<Tooltip label="Edit {coupon.code}">
							<a
								href="/admin/coupons/{coupon.id}"
								class="icon-btn"
								aria-label="Edit {coupon.code}"
							>
								<PencilSimpleIcon size={16} weight="bold" />
							</a>
						</Tooltip>
					</div>
				</div>
			{/each}
		</div>

		<!-- Desktop: table -->
		<div class="cp__twrap">
			<table class="ctable">
				<thead>
					<tr>
						<th>Code</th>
						<th>Discount</th>
						<th class="ctable__num">Usage</th>
						<th>Status</th>
						<th>Active</th>
						<th class="ctable__actions-h" aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each coupons as coupon (coupon.id)}
						{@const st = couponStatus(coupon)}
						<tr>
							<td>
								<a href="/admin/coupons/{coupon.id}" class="ctable__code">{coupon.code}</a>
							</td>
							<td>{formatDiscount(coupon)}</td>
							<td class="ctable__num">{formatUsage(coupon)}</td>
							<td><span class="badge badge--{st}">{statusLabel(st)}</span></td>
							<td>
								<button
									type="button"
									class="toggle-btn"
									onclick={() => toggleActive(coupon)}
									disabled={togglingId === coupon.id}
									title={coupon.is_active ? 'Deactivate' : 'Activate'}
									aria-label={coupon.is_active ? 'Deactivate coupon' : 'Activate coupon'}
								>
									{#if coupon.is_active}
										<ToggleRightIcon size={22} weight="fill" />
									{:else}
										<ToggleLeftIcon size={22} weight="bold" />
									{/if}
								</button>
							</td>
							<td class="ctable__actions">
								<a
									href="/admin/coupons/{coupon.id}"
									class="icon-btn"
									title="Edit {coupon.code}"
									aria-label="Edit {coupon.code}"
								>
									<PencilSimpleIcon size={16} weight="bold" />
								</a>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if totalPages > 1}
		<nav class="cp__pag" aria-label="Pagination">
			<button
				type="button"
				class="page-btn"
				onclick={() => {
					page--;
					loadCoupons();
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
					loadCoupons();
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
	.cp {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ── Header ─────────────────────────── */
	.cp__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: flex-start;
	}

	.cp__heading {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}

	.cp__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.cp__title {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		font-family: var(--font-heading);
		line-height: 1.2;
		letter-spacing: -0.01em;
		margin: 0;
	}

	.cp__subtitle {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
	}

	.cp__actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	/* ── Buttons ────────────────────────── */
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 3rem;
		padding: 0.55rem 1rem;
		font-size: 0.875rem;
		font-weight: 600;
		font-family: var(--font-ui);
		border-radius: var(--radius-2xl);
		text-decoration: none;
		cursor: pointer;
		border: 1px solid transparent;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			transform 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out),
			color 150ms var(--ease-out);
	}

	.btn--primary {
		color: var(--color-white);
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
	}

	.btn--primary:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 10px 22px -4px rgba(15, 164, 175, 0.55);
	}

	.btn--secondary {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.1);
	}

	.btn--secondary:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* ── Bulk form ──────────────────────── */
	.bulk-form {
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(212, 168, 67, 0.25);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.bulk-form__head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
		color: var(--color-gold);
	}

	.bulk-form__title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0;
		font-family: var(--font-heading);
	}

	.bulk-form__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.75rem;
	}

	.bf-field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.bf-label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.bf-input {
		min-height: 3rem;
		padding: 0 1.25rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: 0.875rem;
		font-family: var(--font-ui);
		outline: none;
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.bf-input::placeholder {
		color: var(--color-grey-500);
	}

	.bf-input:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.bf-input--select {
		appearance: none;
		-webkit-appearance: none;
		-moz-appearance: none;
		padding-right: 2.25rem;
		cursor: pointer;
		width: 100%;
	}

	.bf-input--select option {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		color: var(--color-white);
	}

	.bulk-form__foot {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
		margin-top: 1rem;
	}

	/* ── KPIs ───────────────────────────── */
	.kpi-grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.75rem;
	}

	.kpi {
		display: flex;
		align-items: center;
		gap: 0.85rem;
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.kpi--skeleton {
		min-height: 4.25rem;
	}

	.kpi__icon-skel {
		width: 2.75rem;
		height: 2.75rem;
		border-radius: var(--radius-2xl);
		background: rgba(255, 255, 255, 0.06);
		animation: shimmer 1.5s infinite;
		flex-shrink: 0;
	}

	.kpi__text-skel {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		flex: 1;
	}

	.skel {
		height: 0.75rem;
		border-radius: var(--radius-sm);
		background: rgba(255, 255, 255, 0.06);
		animation: shimmer 1.5s infinite;
	}

	.skel--short {
		width: 40%;
	}

	.skel--long {
		width: 65%;
		height: 1rem;
	}

	.skel--full {
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
		border-radius: var(--radius-2xl);
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
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
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
		min-height: 3rem;
		padding: 0 1.25rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
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
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		color: var(--color-white);
	}

	/* ── Skel row / empty ───────────────── */
	.cp__skel {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.skelrow {
		padding: 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		gap: 0.85rem;
		padding: 3.5rem 2rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
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
	}

	.badge--active {
		background-color: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}

	.badge--expired {
		background-color: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
	}

	.badge--inactive {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	/* ── Toggle / icon-btn ──────────────── */
	.toggle-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.25rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--color-teal-light);
		border-radius: var(--radius-md);
		transition:
			background-color 150ms var(--ease-out),
			opacity 150ms var(--ease-out);
	}

	.toggle-btn:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.05);
	}

	.toggle-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

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
		text-decoration: none;
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

	/* ── Mobile cards ───────────────────── */
	.cp__cards {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.cp__twrap {
		display: none;
	}

	.ccard {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
	}

	.ccard__top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.ccard__code {
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 0.875rem;
		color: var(--color-white);
		text-decoration: none;
		letter-spacing: 0.04em;
	}

	.ccard__code:hover {
		color: var(--color-teal-light);
	}

	.ccard__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.ccard__label {
		font-size: 0.75rem;
		color: var(--color-grey-500);
	}

	.ccard__value {
		font-size: 0.875rem;
		color: var(--color-grey-300);
		font-variant-numeric: tabular-nums;
	}

	.ccard__foot {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: 0.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	/* ── Pagination ─────────────────────── */
	.cp__pag {
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
		border-radius: var(--radius-2xl);
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

	/* ── Tablet 480px+ ──────────────────── */
	@media (min-width: 480px) {
		.kpi-grid {
			grid-template-columns: repeat(3, 1fr);
		}

		.bulk-form__grid {
			grid-template-columns: 1fr 1fr;
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
		.cp {
			gap: 1.5rem;
		}

		.cp__header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
		}


		.kpi-grid {
			gap: 1rem;
		}

		.kpi {
			padding: 1.5rem;
		}


		.bulk-form {
			padding: 1.5rem;
		}

		.bulk-form__grid {
			grid-template-columns: repeat(3, 1fr);
		}

		.filter-card {
			padding: 1.5rem;
		}

		.cp__cards {
			display: none;
		}

		.cp__twrap {
			display: block;
			overflow-x: auto;
			background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-2xl);
			box-shadow:
				0 1px 0 rgba(255, 255, 255, 0.03) inset,
				0 12px 32px rgba(0, 0, 0, 0.18);
		}

		.ctable {
			width: 100%;
			border-collapse: collapse;
			min-width: 640px;
		}

		.ctable thead {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.ctable th {
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

		.ctable td {
			padding: 0.875rem 1rem;
			font-size: 0.875rem;
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			line-height: 1.45;
		}

		.ctable tbody tr {
			transition: background-color 150ms var(--ease-out);
		}

		.ctable tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.ctable tbody tr:last-child td {
			border-bottom: none;
		}

		.ctable__code {
			font-family: var(--font-mono);
			font-weight: 600;
			color: var(--color-white);
			text-decoration: none;
			letter-spacing: 0.04em;
		}

		.ctable__code:hover {
			color: var(--color-teal-light);
		}

		.ctable__num {
			text-align: right;
			font-variant-numeric: tabular-nums;
			white-space: nowrap;
		}

		.ctable__actions-h,
		.ctable__actions {
			width: 3rem;
			text-align: right;
		}

		.cp__pag {
			gap: 1rem;
		}
	}
</style>
