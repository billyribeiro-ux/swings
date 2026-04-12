<script lang="ts">
	import { api } from '$lib/api/client';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import Plus from 'phosphor-svelte/lib/Plus';
	import Ticket from 'phosphor-svelte/lib/Ticket';
	import PencilSimple from 'phosphor-svelte/lib/PencilSimple';
	import ToggleLeft from 'phosphor-svelte/lib/ToggleLeft';
	import ToggleRight from 'phosphor-svelte/lib/ToggleRight';

	interface Coupon {
		id: string;
		code: string;
		discount_type: 'percentage' | 'fixed' | 'free_trial';
		value: number;
		usage_count: number;
		usage_limit: number | null;
		active: boolean;
		expiry_date: string | null;
		created_at: string;
	}

	interface CouponsResponse {
		data: Coupon[];
		total: number;
		page: number;
		per_page: number;
		total_pages: number;
	}

	let coupons: Coupon[] = $state([]);
	let total = $state(0);
	let currentPage = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let search = $state('');
	let filter: 'all' | 'active' | 'expired' | 'inactive' = $state('all');
	let togglingId = $state('');
	let searchTimeout: ReturnType<typeof setTimeout>;

	$effect(() => {
		loadCoupons();
	});

	async function loadCoupons() {
		loading = true;
		try {
			let url = `/api/admin/coupons?page=${currentPage}&per_page=20`;
			if (filter === 'active') url += '&active=true&expired=false';
			else if (filter === 'expired') url += '&expired=true';
			else if (filter === 'inactive') url += '&active=false';
			if (search) url += `&search=${encodeURIComponent(search)}`;
			const res = await api.get<CouponsResponse>(url);
			coupons = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch (e) {
			console.error('Failed to load coupons', e);
		} finally {
			loading = false;
		}
	}

	function handleSearch(e: Event) {
		const val = (e.target as HTMLInputElement).value;
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			search = val;
			currentPage = 1;
			loadCoupons();
		}, 300);
	}

	function changeFilter(f: 'all' | 'active' | 'expired' | 'inactive') {
		filter = f;
		currentPage = 1;
		loadCoupons();
	}

	async function toggleActive(coupon: Coupon) {
		togglingId = coupon.id;
		try {
			await api.put(`/api/admin/coupons/${coupon.id}`, { active: !coupon.active });
			coupon.active = !coupon.active;
			coupons = [...coupons];
		} catch (e) {
			console.error('Failed to toggle coupon', e);
		} finally {
			togglingId = '';
		}
	}

	function formatDiscount(c: Coupon): string {
		if (c.discount_type === 'percentage') return `${c.value}%`;
		if (c.discount_type === 'fixed') return `$${c.value.toFixed(2)}`;
		return `${c.value} day trial`;
	}

	function statusLabel(c: Coupon): string {
		if (c.expiry_date && new Date(c.expiry_date) < new Date()) return 'Expired';
		return c.active ? 'Active' : 'Inactive';
	}

	function statusClass(c: Coupon): string {
		const s = statusLabel(c);
		if (s === 'Active') return 'badge--active';
		if (s === 'Expired') return 'badge--expired';
		return 'badge--inactive';
	}
</script>

<svelte:head>
	<title>Coupons -- Admin -- Explosive Swings</title>
</svelte:head>

<div class="cpn-list">
	<div class="cpn-list__header">
		<div>
			<h1 class="cpn-list__title">Coupons</h1>
			<p class="cpn-list__subtitle">Manage discount codes and promotions</p>
		</div>
		<a href="/admin/coupons/new" class="cpn-list__create-btn">
			<Plus size={18} weight="bold" />
			Create Coupon
		</a>
	</div>

	<div class="cpn-list__filters">
		<div class="cpn-list__tabs">
			{#each [['all', 'All'], ['active', 'Active'], ['expired', 'Expired'], ['inactive', 'Inactive']] as [key, label] (key)}
				<button class:active={filter === key} onclick={() => changeFilter(key as typeof filter)}>
					{label}
				</button>
			{/each}
		</div>
		<div class="cpn-list__search-wrap">
			<MagnifyingGlass size={16} weight="bold" class="cpn-search-icon" />
			<input
				type="search"
				class="cpn-list__search"
				placeholder="Search coupons..."
				oninput={handleSearch}
			/>
		</div>
	</div>

	{#if loading}
		<div class="cpn-list__loading">
			{#each Array(5) as _, i (i)}
				<div class="skeleton-row"></div>
			{/each}
		</div>
	{:else if coupons.length === 0}
		<div class="cpn-list__empty">
			<div class="cpn-list__empty-icon">
				<Ticket size={48} weight="duotone" />
			</div>
			<h2 class="cpn-list__empty-title">No coupons found</h2>
			<p class="cpn-list__empty-desc">
				{#if search || filter !== 'all'}
					Try adjusting your search or filters.
				{:else}
					Create your first coupon to get started.
				{/if}
			</p>
			{#if !search && filter === 'all'}
				<a href="/admin/coupons/new" class="cpn-list__create-btn">
					<Plus size={18} weight="bold" />
					Create Coupon
				</a>
			{/if}
		</div>
	{:else}
		<!-- Mobile cards -->
		<div class="cpn-list__cards">
			{#each coupons as coupon (coupon.id)}
				<div class="cpn-card">
					<div class="cpn-card__top">
						<span class="cpn-card__code">{coupon.code}</span>
						<span class="cpn-badge {statusClass(coupon)}">{statusLabel(coupon)}</span>
					</div>
					<div class="cpn-card__info">
						<span class="cpn-card__discount">{formatDiscount(coupon)}</span>
						<span class="cpn-card__usage">
							{coupon.usage_count}{coupon.usage_limit != null ? `/${coupon.usage_limit}` : ''} used
						</span>
					</div>
					<div class="cpn-card__actions">
						<button
							class="cpn-card__toggle-btn"
							disabled={togglingId === coupon.id}
							onclick={() => toggleActive(coupon)}
							title={coupon.active ? 'Deactivate' : 'Activate'}
						>
							{#if coupon.active}
								<ToggleRight size={22} weight="fill" />
							{:else}
								<ToggleLeft size={22} />
							{/if}
						</button>
						<a href="/admin/coupons/{coupon.id}" class="cpn-card__edit-link">
							<PencilSimple size={16} />
							Edit
						</a>
					</div>
				</div>
			{/each}
		</div>

		<!-- Desktop table -->
		<div class="cpn-list__table-wrap">
			<table class="cpn-list__table">
				<thead>
					<tr>
						<th>Code</th>
						<th>Discount</th>
						<th>Usage</th>
						<th>Status</th>
						<th>Toggle</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each coupons as coupon (coupon.id)}
						<tr>
							<td>
								<span class="cpn-code-cell">{coupon.code}</span>
							</td>
							<td class="cpn-discount-cell">{formatDiscount(coupon)}</td>
							<td class="cpn-usage-cell">
								{coupon.usage_count}{coupon.usage_limit != null ? ` / ${coupon.usage_limit}` : ''}
							</td>
							<td>
								<span class="cpn-badge {statusClass(coupon)}">{statusLabel(coupon)}</span>
							</td>
							<td>
								<button
									class="cpn-toggle-btn"
									class:cpn-toggle-btn--on={coupon.active}
									disabled={togglingId === coupon.id}
									onclick={() => toggleActive(coupon)}
									title={coupon.active ? 'Deactivate' : 'Activate'}
								>
									{#if coupon.active}
										<ToggleRight size={20} weight="fill" />
									{:else}
										<ToggleLeft size={20} />
									{/if}
								</button>
							</td>
							<td>
								<a href="/admin/coupons/{coupon.id}" class="cpn-edit-link">
									<PencilSimple size={14} />
									Edit
								</a>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<div class="cpn-list__pagination">
				<button
					disabled={currentPage <= 1}
					onclick={() => { currentPage--; loadCoupons(); }}
				>Prev</button>
				<span>Page {currentPage} of {totalPages} ({total} coupons)</span>
				<button
					disabled={currentPage >= totalPages}
					onclick={() => { currentPage++; loadCoupons(); }}
				>Next</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	.cpn-list__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	.cpn-list__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
	}
	.cpn-list__subtitle {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin: 0.25rem 0 0;
	}
	.cpn-list__create-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 1.15rem;
		border-radius: var(--radius-lg);
		background: var(--color-teal);
		color: #fff;
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		text-decoration: none;
		border: none;
		cursor: pointer;
		white-space: nowrap;
		transition: opacity var(--duration-150) var(--ease-out), transform var(--duration-150) var(--ease-out);
	}
	.cpn-list__create-btn:hover {
		opacity: 0.9;
		transform: translateY(-1px);
	}
	/* Filters */
	.cpn-list__filters {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}
	.cpn-list__tabs {
		display: flex;
		gap: 0.25rem;
	}
	.cpn-list__tabs button {
		padding: 0.4rem 0.75rem;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
	}
	.cpn-list__tabs button:hover { color: var(--color-white); }
	.cpn-list__tabs button.active {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}
	.cpn-list__search-wrap { position: relative; }
	:global(.cpn-search-icon) {
		position: absolute;
		left: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500) !important;
		pointer-events: none;
	}
	.cpn-list__search {
		width: 100%;
		padding: 0.55rem 0.75rem 0.55rem 2.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		background: rgba(0, 0, 0, 0.2);
		color: var(--color-white);
		font-size: var(--fs-sm);
		outline: none;
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.cpn-list__search:focus { border-color: var(--color-teal); }
	.cpn-list__search::placeholder { color: var(--color-grey-500); }
	/* Loading */
	.cpn-list__loading {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.skeleton-row {
		height: 3rem;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
		animation: pulse 1.5s ease-in-out infinite;
	}
	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}
	/* Empty */
	.cpn-list__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: 4rem 2rem;
	}
	.cpn-list__empty-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 5rem;
		height: 5rem;
		border-radius: var(--radius-2xl);
		background: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-500);
		margin-bottom: 1.5rem;
	}
	.cpn-list__empty-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0 0 0.5rem;
	}
	.cpn-list__empty-desc {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin: 0 0 1.5rem;
		max-width: 24rem;
	}
	/* Badges */
	.cpn-badge {
		display: inline-block;
		padding: 0.15rem 0.55rem;
		border-radius: var(--radius-md);
		font-size: var(--fs-2xs);
		font-weight: var(--w-semibold);
		white-space: nowrap;
	}
	.badge--active {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}
	.badge--expired {
		background: rgba(212, 168, 67, 0.15);
		color: var(--color-gold);
	}
	.badge--inactive {
		background: rgba(148, 163, 184, 0.15);
		color: #94a3b8;
	}
	/* Mobile cards */
	.cpn-list__cards {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.cpn-list__table-wrap { display: none; }
	.cpn-card {
		padding: 1rem;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}
	.cpn-card__top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}
	.cpn-card__code {
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
		color: var(--color-white);
		letter-spacing: 0.05em;
	}
	.cpn-card__info {
		display: flex;
		gap: 1rem;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-bottom: 0.75rem;
	}
	.cpn-card__discount { color: var(--color-teal-light); font-weight: var(--w-semibold); }
	.cpn-card__actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.cpn-card__toggle-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--color-grey-400);
		padding: 0.25rem;
		transition: color 150ms var(--ease-out);
	}
	.cpn-card__toggle-btn:hover { color: var(--color-teal); }
	.cpn-card__edit-link {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.35rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		text-decoration: none;
		transition: all 150ms var(--ease-out);
	}
	.cpn-card__edit-link:hover {
		border-color: var(--color-teal);
		color: var(--color-teal-light);
	}
	/* Table */
	.cpn-code-cell {
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-weight: var(--w-bold);
		color: var(--color-white);
		letter-spacing: 0.04em;
	}
	.cpn-discount-cell {
		color: var(--color-teal-light);
		font-weight: var(--w-semibold);
	}
	.cpn-usage-cell {
		color: var(--color-grey-300);
	}
	.cpn-toggle-btn {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--color-grey-500);
		padding: 0.25rem;
		transition: color 150ms var(--ease-out);
	}
	.cpn-toggle-btn--on { color: var(--color-teal); }
	.cpn-toggle-btn:hover { color: var(--color-teal-light); }
	.cpn-toggle-btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.cpn-edit-link {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		text-decoration: none;
		transition: color 150ms var(--ease-out);
	}
	.cpn-edit-link:hover { color: var(--color-teal-light); }
	/* Pagination */
	.cpn-list__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1.5rem;
	}
	.cpn-list__pagination button {
		padding: 0.4rem 0.85rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
	}
	.cpn-list__pagination button:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.15);
	}
	.cpn-list__pagination button:disabled { opacity: 0.3; cursor: not-allowed; }
	.cpn-list__pagination span {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	@media (min-width: 768px) {
		.cpn-list__header {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 2rem;
		}
		.cpn-list__title { font-size: var(--fs-2xl); }
		.cpn-list__filters {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 1.5rem;
		}
		.cpn-list__search-wrap { width: 16rem; }
		.cpn-list__cards { display: none; }
		.cpn-list__table-wrap {
			display: block;
			overflow-x: auto;
			background: rgba(255, 255, 255, 0.02);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
		}
		.cpn-list__table {
			width: 100%;
			border-collapse: collapse;
		}
		.cpn-list__table th {
			text-align: left;
			padding: 0.75rem 1rem;
			font-size: var(--fs-xs);
			font-weight: var(--w-semibold);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			color: var(--color-grey-400);
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}
		.cpn-list__table td {
			padding: 0.85rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			font-size: var(--fs-sm);
			color: var(--color-grey-200);
			vertical-align: middle;
		}
		.cpn-list__table tbody tr {
			transition: background var(--duration-150) var(--ease-out);
		}
		.cpn-list__table tbody tr:hover {
			background: rgba(255, 255, 255, 0.02);
		}
	}
</style>
