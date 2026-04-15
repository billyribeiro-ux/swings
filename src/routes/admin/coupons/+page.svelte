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

	interface CouponStats { active_count: number; total_usages: number; total_discount_cents: number; }
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
		{ label: 'All', value: 'all' }, { label: 'Active', value: 'active' },
		{ label: 'Expired', value: 'expired' }, { label: 'Inactive', value: 'inactive' }
	];

	function handleSearchInput(value: string) {
		search = value;
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => { page = 1; loadCoupons(); }, 300);
	}

	function setFilter(tab: FilterTab) { filter = tab; page = 1; loadCoupons(); }

	async function loadStats() {
		statsLoading = true;
		try { stats = await api.get<CouponStats>('/api/admin/coupons/stats'); }
		catch { /* silent */ } finally { statsLoading = false; }
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
			coupons = res.data; total = res.total; totalPages = res.total_pages;
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
				count: bulkCount, prefix: bulkPrefix || undefined, discount_type: bulkDiscountType,
				discount_value: bulkDiscountValue, usage_limit: bulkUsageLimit, expires_at: bulkExpiresAt || undefined
			};
			await api.post('/api/admin/coupons/bulk', payload);
			showBulkForm = false;
			bulkCount = 10; bulkPrefix = ''; bulkDiscountValue = 10; bulkUsageLimit = undefined; bulkExpiresAt = '';
			await Promise.all([loadCoupons(), loadStats()]);
		} catch { alert('Failed to generate coupons'); } finally { bulkLoading = false; }
	}

	onMount(() => { loadStats(); loadCoupons(); });

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
	function statusLabel(s: string): string { return s.charAt(0).toUpperCase() + s.slice(1); }
	function formatMoney(cents: number): string {
		return '$' + (cents / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	}
</script>

<svelte:head><title>Coupons - Admin - Precision Options Signals</title></svelte:head>

<div class="cp">
	<div class="cp__header">
		<div>
			<h1 class="cp__title">Coupons</h1>
			<p class="cp__count">{total} total coupons</p>
		</div>
		<div class="cp__actions">
			<button class="cp__bulk-btn" onclick={() => (showBulkForm = !showBulkForm)}>
				<Lightning size={16} weight="bold" /> Bulk Generate
			</button>
			<a href="/admin/coupons/new" class="cp__cta"><Plus size={16} weight="bold" /> Create Coupon</a>
		</div>
	</div>

	{#if showBulkForm}
		<form class="bf" onsubmit={(e) => { e.preventDefault(); submitBulk(); }}>
			<h3 class="bf__title">Bulk Generate Coupons</h3>
			<div class="bf__grid">
				<label class="bf__field"><span class="bf__lbl">Count</span>
					<input type="number" min="1" max="500" bind:value={bulkCount} class="bf__inp" required /></label>
				<label class="bf__field"><span class="bf__lbl">Code Prefix</span>
					<input type="text" placeholder="e.g. SUMMER" bind:value={bulkPrefix} class="bf__inp" /></label>
				<label class="bf__field"><span class="bf__lbl">Discount Type</span>
					<select bind:value={bulkDiscountType} class="bf__inp">
						<option value="percentage">Percentage</option><option value="fixed_amount">Fixed Amount</option>
					</select></label>
				<label class="bf__field"><span class="bf__lbl">Value {bulkDiscountType === 'percentage' ? '(%)' : '(cents)'}</span>
					<input type="number" min="1" bind:value={bulkDiscountValue} class="bf__inp" required /></label>
				<label class="bf__field"><span class="bf__lbl">Usage Limit</span>
					<input type="number" min="1" placeholder="Unlimited" bind:value={bulkUsageLimit} class="bf__inp" /></label>
				<label class="bf__field"><span class="bf__lbl">Expires At</span>
					<input type="date" bind:value={bulkExpiresAt} class="bf__inp" /></label>
			</div>
			<div class="bf__foot">
				<button type="button" class="bf__cancel" onclick={() => (showBulkForm = false)}>Cancel</button>
				<button type="submit" class="bf__submit" disabled={bulkLoading}>{bulkLoading ? 'Generating...' : `Generate ${bulkCount} Coupons`}</button>
			</div>
		</form>
	{/if}

	{#if statsLoading}
		<div class="cp__kpis">{#each Array(3) as _, i (i)}<div class="kpi kpi--skel"><div class="kpi__iskel"></div><div class="kpi__tskel"><div class="skel skel--s"></div><div class="skel skel--l"></div></div></div>{/each}</div>
	{:else if stats}
		<div class="cp__kpis">
			<div class="kpi"><div class="kpi__ic kpi__ic--green"><Ticket size={22} weight="fill" /></div><div><p class="kpi__lbl">Active Coupons</p><p class="kpi__val">{stats.active_count.toLocaleString()}</p></div></div>
			<div class="kpi"><div class="kpi__ic kpi__ic--blue"><ChartBar size={22} weight="fill" /></div><div><p class="kpi__lbl">Total Usages</p><p class="kpi__val">{stats.total_usages.toLocaleString()}</p></div></div>
			<div class="kpi"><div class="kpi__ic kpi__ic--teal"><CurrencyDollar size={22} weight="fill" /></div><div><p class="kpi__lbl">Total Discount</p><p class="kpi__val">{formatMoney(stats.total_discount_cents)}</p></div></div>
		</div>
	{/if}

	<div class="cp__toolbar">
		<div class="cp__tabs">{#each filters as tab (tab.value)}<button class="cp__tab" class:cp__tab--on={filter === tab.value} onclick={() => setFilter(tab.value)}>{tab.label}</button>{/each}</div>
		<div class="cp__search"><MagnifyingGlass size={18} weight="bold" /><input type="text" placeholder="Search by code..." value={search} oninput={(e) => handleSearchInput(e.currentTarget.value)} class="cp__sinput" /></div>
	</div>

	{#if loading}
		<div class="cp__skel">{#each Array(6) as _, i (i)}<div class="skelrow"><div class="skel skel--f"></div></div>{/each}</div>
	{:else if coupons.length === 0}
		<div class="cp__empty"><p>No coupons found.</p></div>
	{:else}
		<div class="cp__cards">{#each coupons as coupon (coupon.id)}{@const st = couponStatus(coupon)}
			<div class="cc">
				<div class="cc__top"><a href="/admin/coupons/{coupon.id}" class="cc__code">{coupon.code}</a><span class="badge badge--{st}">{statusLabel(st)}</span></div>
				<div class="cc__row"><span class="cc__lbl">Discount</span><span class="cc__val">{formatDiscount(coupon)}</span></div>
				<div class="cc__row"><span class="cc__lbl">Usage</span><span class="cc__val">{formatUsage(coupon)}</span></div>
				<div class="cc__foot">
					<button class="tgl" onclick={() => toggleActive(coupon)} disabled={togglingId === coupon.id} title={coupon.is_active ? 'Deactivate' : 'Activate'}>{#if coupon.is_active}<ToggleRight size={24} weight="fill" />{:else}<ToggleLeft size={24} weight="bold" />{/if}</button>
					<a href="/admin/coupons/{coupon.id}" class="cc__edit">Edit</a>
				</div>
			</div>{/each}</div>
		<div class="cp__twrap">
			<table class="ct"><thead><tr><th>Code</th><th>Discount</th><th>Usage</th><th>Status</th><th>Active</th><th>Actions</th></tr></thead>
				<tbody>{#each coupons as coupon (coupon.id)}{@const st = couponStatus(coupon)}
					<tr>
						<td><a href="/admin/coupons/{coupon.id}" class="ct__code">{coupon.code}</a></td>
						<td>{formatDiscount(coupon)}</td><td>{formatUsage(coupon)}</td>
						<td><span class="badge badge--{st}">{statusLabel(st)}</span></td>
						<td><button class="tgl" onclick={() => toggleActive(coupon)} disabled={togglingId === coupon.id} title={coupon.is_active ? 'Deactivate' : 'Activate'}>{#if coupon.is_active}<ToggleRight size={22} weight="fill" />{:else}<ToggleLeft size={22} weight="bold" />{/if}</button></td>
						<td><a href="/admin/coupons/{coupon.id}" class="ct__link">Edit</a></td>
					</tr>{/each}</tbody></table></div>
	{/if}

	{#if totalPages > 1}
		<div class="cp__pag">
			<button onclick={() => { page--; loadCoupons(); }} disabled={page <= 1} class="cp__pbtn"><CaretLeft size={16} weight="bold" /> Prev</button>
			<span class="cp__pinfo">Page {page} of {totalPages}</span>
			<button onclick={() => { page++; loadCoupons(); }} disabled={page >= totalPages} class="cp__pbtn">Next <CaretRight size={16} weight="bold" /></button>
		</div>
	{/if}
</div>

<style>
	.cp__header { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.25rem; }
	.cp__title { font-size: var(--fs-xl); font-weight: var(--w-bold); color: var(--color-white); font-family: var(--font-heading); }
	.cp__count { font-size: var(--fs-xs); color: var(--color-grey-400); margin-top: 0.15rem; }
	.cp__actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }
	.cp__cta, .cp__bulk-btn { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.55rem 1rem; font-size: var(--fs-xs); font-weight: var(--w-semibold); border-radius: var(--radius-lg); cursor: pointer; }
	.cp__cta { color: var(--color-white); background: linear-gradient(135deg, var(--color-teal), #0d8a94); text-decoration: none; border: none; transition: opacity 200ms var(--ease-out), transform 200ms var(--ease-out); }
	.cp__cta:hover { opacity: 0.9; transform: translateY(-1px); }
	.cp__bulk-btn { color: var(--color-gold); background: rgba(212,168,67,0.1); border: 1px solid rgba(212,168,67,0.25); transition: background-color 200ms var(--ease-out), border-color 200ms var(--ease-out); }
	.cp__bulk-btn:hover { background: rgba(212,168,67,0.2); border-color: rgba(212,168,67,0.4); }
	.bf { padding: 1rem; margin-bottom: 1.25rem; background-color: var(--color-navy-mid); border: 1px solid rgba(212,168,67,0.2); border-radius: var(--radius-xl); }
	.bf__title { font-size: var(--fs-sm); font-weight: var(--w-semibold); color: var(--color-gold); margin-bottom: 0.75rem; }
	.bf__grid { display: grid; grid-template-columns: 1fr; gap: 0.75rem; }
	.bf__field { display: flex; flex-direction: column; gap: 0.25rem; }
	.bf__lbl { font-size: var(--fs-xs); color: var(--color-grey-400); }
	.bf__inp { padding: 0.55rem 0.75rem; background-color: var(--color-navy); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-md); color: var(--color-white); font-size: var(--fs-sm); font-family: var(--font-ui); }
	.bf__inp:focus { outline: none; border-color: var(--color-teal); }
	.bf__foot { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem; }
	.bf__cancel { padding: 0.5rem 1rem; font-size: var(--fs-xs); color: var(--color-grey-400); background: none; border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-lg); cursor: pointer; }
	.bf__cancel:hover { border-color: rgba(255,255,255,0.2); color: var(--color-white); }
	.bf__submit { padding: 0.5rem 1.25rem; font-size: var(--fs-xs); font-weight: var(--w-semibold); color: var(--color-white); background: linear-gradient(135deg, var(--color-gold), #b8912e); border: none; border-radius: var(--radius-lg); cursor: pointer; transition: opacity 200ms var(--ease-out); }
	.bf__submit:hover { opacity: 0.9; } .bf__submit:disabled { opacity: 0.5; cursor: not-allowed; }
	.cp__kpis { display: grid; grid-template-columns: 1fr; gap: 0.75rem; margin-bottom: 1.25rem; }
	.kpi { display: flex; align-items: center; gap: 0.75rem; padding: 1rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-xl); }
	.kpi--skel { min-height: 4rem; }
	.kpi__iskel { width: 2.5rem; height: 2.5rem; border-radius: var(--radius-lg); background: rgba(255,255,255,0.06); animation: shimmer 1.5s infinite; flex-shrink: 0; }
	.kpi__tskel { display: flex; flex-direction: column; gap: 0.4rem; flex: 1; }
	.skel { height: 0.75rem; border-radius: var(--radius-sm); background: rgba(255,255,255,0.06); animation: shimmer 1.5s infinite; }
	.skel--s { width: 40%; } .skel--l { width: 65%; height: 1rem; } .skel--f { width: 100%; height: 2.5rem; }
	@keyframes shimmer { 0%, 100% { opacity: 0.4; } 50% { opacity: 0.8; } }
	.kpi__ic { width: 2.5rem; height: 2.5rem; border-radius: var(--radius-lg); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
	.kpi__ic--green { background-color: rgba(34,197,94,0.15); color: #22c55e; }
	.kpi__ic--blue { background-color: rgba(59,130,246,0.15); color: #3b82f6; }
	.kpi__ic--teal { background-color: rgba(15,164,175,0.15); color: var(--color-teal); }
	.kpi__lbl { font-size: var(--fs-xs); color: var(--color-grey-400); margin-bottom: 0.1rem; }
	.kpi__val { font-size: var(--fs-md); font-weight: var(--w-bold); color: var(--color-white); }
	.cp__toolbar { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.25rem; }
	.cp__tabs { display: flex; gap: 0.25rem; overflow-x: auto; }
	.cp__tab { padding: 0.5rem 0.85rem; font-size: var(--fs-xs); font-weight: var(--w-medium); color: var(--color-grey-400); background: none; border: 1px solid transparent; border-radius: var(--radius-lg); cursor: pointer; white-space: nowrap; transition: color 200ms var(--ease-out), background-color 200ms var(--ease-out), border-color 200ms var(--ease-out); }
	.cp__tab:hover { color: var(--color-white); background-color: rgba(255,255,255,0.04); }
	.cp__tab--on { color: var(--color-teal-light); background-color: var(--color-teal-glow); border-color: rgba(15,164,175,0.3); }
	.cp__search { display: flex; align-items: center; gap: 0.5rem; padding: 0.65rem 0.85rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-lg); color: var(--color-grey-400); transition: border-color 200ms var(--ease-out); }
	.cp__search:focus-within { border-color: var(--color-teal); }
	.cp__sinput { flex: 1; background: none; border: none; outline: none; color: var(--color-white); font-size: var(--fs-sm); font-family: var(--font-ui); }
	.cp__sinput::placeholder { color: var(--color-grey-500); }
	.cp__skel { display: flex; flex-direction: column; gap: 0.5rem; }
	.skelrow { padding: 1rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-lg); }
	.cp__empty { text-align: center; padding: 3rem 1rem; color: var(--color-grey-400); font-size: var(--fs-sm); background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-xl); }
	.badge { display: inline-block; font-size: var(--fs-xs); font-weight: var(--w-semibold); padding: 0.15rem 0.55rem; border-radius: var(--radius-full); }
	.badge--active { background-color: rgba(34,181,115,0.12); color: var(--color-green); }
	.badge--expired { background-color: rgba(224,72,72,0.12); color: var(--color-red); }
	.badge--inactive { background-color: rgba(255,255,255,0.06); color: var(--color-grey-400); }
	.tgl { background: none; border: none; cursor: pointer; padding: 0.15rem; display: flex; align-items: center; transition: opacity 200ms var(--ease-out); }
	.tgl:disabled { opacity: 0.4; cursor: not-allowed; } .tgl :global(svg) { color: var(--color-teal); }
	.cp__cards { display: flex; flex-direction: column; gap: 0.5rem; }
	.cp__twrap { display: none; }
	.cc { display: flex; flex-direction: column; gap: 0.5rem; padding: 0.85rem 1rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-lg); }
	.cc__top { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
	.cc__code, .ct__code { font-family: 'Courier New', Courier, monospace; font-weight: var(--w-semibold); color: var(--color-white); text-decoration: none; letter-spacing: 0.04em; }
	.cc__code { font-size: var(--fs-sm); } .cc__code:hover, .ct__code:hover { color: var(--color-teal-light); }
	.cc__row { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
	.cc__lbl { font-size: var(--fs-xs); color: var(--color-grey-400); }
	.cc__val { font-size: var(--fs-sm); color: var(--color-grey-300); }
	.cc__foot { display: flex; justify-content: space-between; align-items: center; padding-top: 0.5rem; border-top: 1px solid rgba(255,255,255,0.06); }
	.cc__edit, .ct__link { font-size: var(--fs-sm); font-weight: var(--w-semibold); color: var(--color-teal-light); text-decoration: none; }
	.cc__edit:hover, .ct__link:hover { text-decoration: underline; }
	.cp__pag { display: flex; align-items: center; justify-content: center; gap: 0.75rem; margin-top: 1rem; }
	.cp__pbtn { display: flex; align-items: center; gap: 0.25rem; padding: 0.5rem 0.75rem; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.1); border-radius: var(--radius-lg); color: var(--color-white); font-size: var(--fs-xs); cursor: pointer; transition: border-color 200ms var(--ease-out); }
	.cp__pbtn:hover:not(:disabled) { border-color: var(--color-teal); }
	.cp__pbtn:disabled { opacity: 0.4; cursor: not-allowed; }
	.cp__pinfo { font-size: var(--fs-xs); color: var(--color-grey-400); }
	@media (min-width: 480px) { .cp__kpis { grid-template-columns: repeat(3, 1fr); } .bf__grid { grid-template-columns: 1fr 1fr; } }
	@media (min-width: 768px) {
		.cp__header { flex-direction: row; justify-content: space-between; align-items: flex-start; margin-bottom: 1.5rem; }
		.cp__title { font-size: var(--fs-2xl); } .cp__count { font-size: var(--fs-sm); margin-top: 0.25rem; }
		.cp__cta, .cp__bulk-btn { padding: 0.6rem 1.25rem; font-size: var(--fs-sm); }
		.cp__kpis { gap: 1rem; margin-bottom: 2rem; } .kpi { padding: 1.15rem; gap: 1rem; }
		.kpi__ic { width: 2.75rem; height: 2.75rem; } .kpi__val { font-size: var(--fs-lg); }
		.cp__toolbar { flex-direction: row; justify-content: space-between; align-items: center; margin-bottom: 1.5rem; }
		.cp__search { min-width: 14rem; } .bf { padding: 1.25rem; }
		.bf__grid { grid-template-columns: repeat(3, 1fr); } .bf__title { font-size: var(--fs-md); }
		.cp__cards { display: none; }
		.cp__twrap { display: block; overflow-x: auto; background-color: var(--color-navy-mid); border: 1px solid rgba(255,255,255,0.06); border-radius: var(--radius-xl); }
		.ct { width: 100%; border-collapse: collapse; }
		.ct th { text-align: left; font-size: var(--fs-xs); font-weight: var(--w-semibold); color: var(--color-grey-400); text-transform: uppercase; letter-spacing: 0.05em; padding: 0.85rem 1rem; border-bottom: 1px solid rgba(255,255,255,0.06); }
		.ct td { padding: 0.85rem 1rem; font-size: var(--fs-sm); color: var(--color-grey-300); border-bottom: 1px solid rgba(255,255,255,0.04); }
		.ct tbody tr:hover { background-color: rgba(255,255,255,0.02); }
		.cp__pag { gap: 1rem; margin-top: 1.5rem; }
		.cp__pbtn { gap: 0.35rem; padding: 0.5rem 1rem; font-size: var(--fs-sm); } .cp__pinfo { font-size: var(--fs-sm); }
	}
</style>
