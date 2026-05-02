<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import type { PaginatedResponse } from '$lib/api/types';
	import PackageIcon from 'phosphor-svelte/lib/PackageIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';

	interface MemberOrderListItem {
		id: string;
		number: string;
		status: string;
		currency: string;
		total_cents: number;
		item_count: number;
		placed_at?: string | null;
		completed_at?: string | null;
		created_at: string;
	}

	const PER_PAGE = 20;

	let envelope = $state<PaginatedResponse<MemberOrderListItem> | null>(null);
	let loading = $state(true);
	let error = $state('');
	let page = $state(1);

	type StatusTone = 'green' | 'gold' | 'teal' | 'red' | 'grey';

	function statusTone(status: string): StatusTone {
		const s = status.toLowerCase();
		if (s === 'completed' || s === 'complete' || s === 'paid') return 'green';
		if (s === 'pending' || s === 'processing' || s === 'awaiting_payment') return 'gold';
		if (s === 'refunded' || s === 'partially_refunded') return 'teal';
		if (s === 'cancelled' || s === 'canceled' || s === 'failed' || s === 'voided') return 'red';
		return 'grey';
	}

	function statusLabel(status: string): string {
		return status.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
	}

	function formatMoney(cents: number, currency: string): string {
		try {
			return new Intl.NumberFormat(undefined, {
				style: 'currency',
				currency: currency.toUpperCase()
			}).format(cents / 100);
		} catch {
			return `${currency.toUpperCase()} ${(cents / 100).toFixed(2)}`;
		}
	}

	function formatDate(iso: string | null | undefined): string {
		if (!iso) return '—';
		try {
			return new Intl.DateTimeFormat(undefined, {
				month: 'short',
				day: 'numeric',
				year: 'numeric'
			}).format(new Date(iso));
		} catch {
			return iso;
		}
	}

	async function load(p: number) {
		loading = true;
		error = '';
		try {
			envelope = await api.get<PaginatedResponse<MemberOrderListItem>>(
				`/api/member/orders?page=${p}&per_page=${PER_PAGE}`
			);
			page = envelope.page;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to load orders';
			envelope = null;
		} finally {
			loading = false;
		}
	}

	function gotoPage(p: number) {
		if (!envelope) return;
		const next = Math.max(1, Math.min(envelope.total_pages || 1, p));
		void load(next);
	}

	onMount(() => {
		void load(1);
	});

	const orders = $derived(envelope?.data ?? []);
	const totalPages = $derived(envelope?.total_pages ?? 1);
</script>

<svelte:head><title>My Orders - Precision Options Signals</title></svelte:head>

<section class="orders">
	<header class="orders__header">
		<h1 class="orders__title">My Orders</h1>
		<p class="orders__sub">Receipts and history for every purchase on your account.</p>
	</header>

	{#if loading && !envelope}
		<p class="orders__loading">Loading your orders…</p>
	{:else if error}
		<div class="orders__error" role="alert">{error}</div>
	{:else if orders.length === 0}
		<div class="empty">
			<div class="empty__icon"><PackageIcon size={36} weight="duotone" /></div>
			<h3 class="empty__title">No orders yet</h3>
			<p class="empty__body">When you make a purchase your receipts will appear here.</p>
			<a href={resolve('/pricing')} class="empty__cta">
				Browse Plans <ArrowRightIcon size={14} weight="bold" />
			</a>
		</div>
	{:else}
		<div class="orders__table-wrap" role="region" aria-label="Order history">
			<table class="orders__table">
				<thead>
					<tr>
						<th scope="col">Order #</th>
						<th scope="col">Status</th>
						<th scope="col">Total</th>
						<th scope="col">Date</th>
						<th scope="col" class="orders__col-actions">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each orders as order (order.id)}
						{@const tone = statusTone(order.status)}
						<tr>
							<th scope="row" class="orders__cell-number">
								<a
									class="orders__num-link"
									href={resolve('/dashboard/account/orders/[id]', {
										id: order.id
									})}
								>
									#{order.number}
								</a>
							</th>
							<td>
								<span
									class="badge badge--{tone}"
									aria-label="Status: {statusLabel(order.status)}"
								>
									<span class="badge__icon" aria-hidden="true">
										{#if tone === 'green'}
											<CheckCircleIcon size={14} weight="fill" />
										{:else if tone === 'gold'}
											<ClockIcon size={14} weight="fill" />
										{:else if tone === 'teal'}
											<WarningIcon size={14} weight="fill" />
										{:else if tone === 'red'}
											<XCircleIcon size={14} weight="fill" />
										{:else}
											<ClockIcon size={14} weight="fill" />
										{/if}
									</span>
									<span>{statusLabel(order.status)}</span>
								</span>
							</td>
							<td class="orders__cell-num">
								{formatMoney(order.total_cents, order.currency)}
							</td>
							<td class="orders__cell-date">
								{formatDate(order.placed_at ?? order.created_at)}
							</td>
							<td class="orders__col-actions">
								<a
									class="orders__view"
									href={resolve('/dashboard/account/orders/[id]', {
										id: order.id
									})}
									aria-label="View order #{order.number}"
								>
									<EyeIcon size={14} weight="bold" />
									View
								</a>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>

			<ul class="orders__cards" role="list">
				{#each orders as order (order.id)}
					{@const tone = statusTone(order.status)}
					<li class="card">
						<div class="card__top">
							<a
								class="card__num"
								href={resolve('/dashboard/account/orders/[id]', { id: order.id })}
							>
								#{order.number}
							</a>
							<span class="badge badge--{tone}">
								<span class="badge__icon" aria-hidden="true">
									{#if tone === 'green'}
										<CheckCircleIcon size={14} weight="fill" />
									{:else if tone === 'gold'}
										<ClockIcon size={14} weight="fill" />
									{:else if tone === 'teal'}
										<WarningIcon size={14} weight="fill" />
									{:else if tone === 'red'}
										<XCircleIcon size={14} weight="fill" />
									{:else}
										<ClockIcon size={14} weight="fill" />
									{/if}
								</span>
								<span>{statusLabel(order.status)}</span>
							</span>
						</div>
						<dl class="card__rows">
							<div>
								<dt>Total</dt>
								<dd class="card__num-val">
									{formatMoney(order.total_cents, order.currency)}
								</dd>
							</div>
							<div>
								<dt>Date</dt>
								<dd>{formatDate(order.placed_at ?? order.created_at)}</dd>
							</div>
						</dl>
						<a
							class="card__view"
							href={resolve('/dashboard/account/orders/[id]', { id: order.id })}
						>
							<EyeIcon size={14} weight="bold" />
							View order
						</a>
					</li>
				{/each}
			</ul>
		</div>

		{#if totalPages > 1}
			<nav class="pager" aria-label="Pagination">
				<button
					type="button"
					class="pager__btn"
					onclick={() => gotoPage(page - 1)}
					disabled={page <= 1 || loading}
					aria-label="Previous page"
				>
					<CaretLeftIcon size={16} weight="bold" />
					<span>Prev</span>
				</button>
				<span class="pager__status">Page {page} of {totalPages}</span>
				<button
					type="button"
					class="pager__btn"
					onclick={() => gotoPage(page + 1)}
					disabled={page >= totalPages || loading}
					aria-label="Next page"
				>
					<span>Next</span>
					<CaretRightIcon size={16} weight="bold" />
				</button>
			</nav>
		{/if}
	{/if}
</section>

<style>
	.orders {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.orders__header {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.orders__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.orders__sub {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.orders__loading {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		padding: 1.5rem 0;
	}

	.orders__error {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}

	.orders__table-wrap {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		overflow: hidden;
	}

	.orders__table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}

	.orders__table thead {
		background-color: rgba(255, 255, 255, 0.02);
	}

	.orders__table th,
	.orders__table td {
		text-align: left;
		padding: 0.85rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		vertical-align: middle;
		color: var(--color-grey-300);
	}

	.orders__table tbody tr:last-child th,
	.orders__table tbody tr:last-child td {
		border-bottom: none;
	}

	.orders__table thead th {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-400);
	}

	.orders__cell-number {
		font-weight: var(--w-semibold);
	}

	.orders__num-link {
		color: var(--color-white);
		text-decoration: none;
		font-family: var(--font-heading);
	}

	.orders__num-link:hover {
		color: var(--color-teal);
	}

	.orders__cell-num {
		font-variant-numeric: tabular-nums;
		color: var(--color-white);
		font-weight: var(--w-semibold);
	}

	.orders__cell-date {
		color: var(--color-grey-400);
		white-space: nowrap;
	}

	.orders__col-actions {
		text-align: right;
		width: 1%;
		white-space: nowrap;
	}

	.orders__view {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.4rem 0.75rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.25);
		text-decoration: none;
		transition: background-color 200ms var(--ease-out);
	}

	.orders__view:hover {
		background-color: rgba(15, 164, 175, 0.2);
	}

	.badge {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.25rem 0.65rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.02em;
		white-space: nowrap;
	}

	.badge__icon {
		display: inline-flex;
		align-items: center;
	}

	.badge--green {
		background-color: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}

	.badge--gold {
		background-color: rgba(212, 168, 67, 0.18);
		color: var(--color-gold);
	}

	.badge--teal {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.badge--red {
		background-color: rgba(224, 72, 72, 0.15);
		color: var(--color-red);
	}

	.badge--grey {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	.orders__cards {
		display: none;
		list-style: none;
		padding: 0;
		margin: 0;
		flex-direction: column;
		gap: 0.6rem;
	}

	.card {
		padding: 0.9rem 1rem;
		background-color: rgba(255, 255, 255, 0.02);
		border: 1px solid rgba(255, 255, 255, 0.05);
		border-radius: var(--radius-lg);
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}

	.card__top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.card__num {
		font-family: var(--font-heading);
		color: var(--color-white);
		text-decoration: none;
		font-weight: var(--w-semibold);
	}

	.card__rows {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
		margin: 0;
	}

	.card__rows dt {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 0.15rem;
	}

	.card__rows dd {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}

	.card__num-val {
		font-variant-numeric: tabular-nums;
		color: var(--color-white);
		font-weight: var(--w-semibold);
	}

	.card__view {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.55rem 0.75rem;
		border-radius: var(--radius-lg);
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
		border: 1px solid rgba(15, 164, 175, 0.25);
		text-decoration: none;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
	}

	.empty {
		text-align: center;
		padding: 3rem 1.5rem;
		border: 1px dashed rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-xl);
		background-color: rgba(255, 255, 255, 0.01);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.65rem;
	}

	.empty__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 4rem;
		height: 4rem;
		border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
	}

	.empty__title {
		font-size: var(--fs-md);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.empty__body {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		max-width: 22rem;
	}

	.empty__cta {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		margin-top: 0.5rem;
		padding: 0.55rem 1.1rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.25);
		border-radius: var(--radius-lg);
		text-decoration: none;
	}

	.pager {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		padding: 0.5rem 0;
	}

	.pager__btn {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.5rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
		transition:
			background-color 200ms var(--ease-out),
			color 200ms var(--ease-out);
	}

	.pager__btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.pager__btn:not(:disabled):hover {
		color: var(--color-white);
		background-color: rgba(15, 164, 175, 0.1);
		border-color: rgba(15, 164, 175, 0.25);
	}

	.pager__status {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
	}

	@media (max-width: 640px) {
		.orders__table {
			display: none;
		}

		.orders__cards {
			display: flex;
			padding: 0.6rem;
		}
	}
</style>
