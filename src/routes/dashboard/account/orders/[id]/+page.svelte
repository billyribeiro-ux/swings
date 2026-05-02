<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import PackageIcon from 'phosphor-svelte/lib/PackageIcon';

	interface OrderRow {
		id: string;
		number: string;
		user_id?: string | null;
		status: string;
		currency: string;
		subtotal_cents: number;
		discount_cents: number;
		tax_cents: number;
		total_cents: number;
		email: string;
		stripe_payment_intent_id?: string | null;
		stripe_customer_id?: string | null;
		placed_at?: string | null;
		completed_at?: string | null;
		created_at: string;
		updated_at: string;
	}

	interface OrderItemRow {
		id: string;
		order_id: string;
		product_id: string;
		variant_id?: string | null;
		sku?: string | null;
		name: string;
		quantity: number;
		unit_price_cents: number;
		line_total_cents: number;
		created_at: string;
	}

	interface OrderRefund {
		id: string;
		amount_cents: number;
		reason?: string | null;
		created_at: string;
	}

	interface OrderTransition {
		from_status: string;
		to_status: string;
		reason?: string | null;
		created_at: string;
	}

	interface OrderDetail {
		order: OrderRow;
		items: OrderItemRow[];
		refunds: OrderRefund[];
		state_log: OrderTransition[];
	}

	let detail = $state<OrderDetail | null>(null);
	let loading = $state(true);
	let error = $state('');
	let notFound = $state(false);
	let showHistory = $state(false);

	const orderId = $derived(page.params.id ?? '');

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

	function formatDateTime(iso: string | null | undefined): string {
		if (!iso) return '—';
		try {
			return new Intl.DateTimeFormat(undefined, {
				month: 'short',
				day: 'numeric',
				year: 'numeric',
				hour: 'numeric',
				minute: '2-digit'
			}).format(new Date(iso));
		} catch {
			return iso;
		}
	}

	async function load() {
		loading = true;
		error = '';
		notFound = false;
		try {
			detail = await api.get<OrderDetail>(
				`/api/member/orders/${encodeURIComponent(orderId)}`
			);
		} catch (e) {
			if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
				notFound = true;
			} else {
				error = e instanceof ApiError ? e.message : 'Failed to load order';
			}
			detail = null;
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		void load();
	});
</script>

<svelte:head>
	<title>Order Detail - Precision Options Signals</title>
</svelte:head>

<section class="od">
	<a class="od__back" href={resolve('/dashboard/account/orders')}>
		<ArrowLeftIcon size={14} weight="bold" />
		Back to orders
	</a>

	{#if loading}
		<p class="od__loading">Loading order…</p>
	{:else if notFound}
		<div class="od__notfound" role="alert">
			<div class="empty__icon"><PackageIcon size={36} weight="duotone" /></div>
			<h2 class="empty__title">Order not found or access denied</h2>
			<p class="empty__body">
				This order may have been deleted, or you may not have permission to view it.
			</p>
			<a class="empty__cta" href={resolve('/dashboard/account/orders')}>
				Back to your orders
			</a>
		</div>
	{:else if error}
		<div class="od__error" role="alert">{error}</div>
	{:else if detail}
		{@const tone = statusTone(detail.order.status)}
		<header class="od__header">
			<div class="od__header-row">
				<div>
					<h1 class="od__num">Order #{detail.order.number}</h1>
					<p class="od__placed">
						Placed {formatDateTime(detail.order.placed_at ?? detail.order.created_at)}
					</p>
				</div>
				<div class="od__header-meta">
					<span class="badge badge--{tone}">
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
						<span>{statusLabel(detail.order.status)}</span>
					</span>
					<span class="od__total">
						{formatMoney(detail.order.total_cents, detail.order.currency)}
					</span>
				</div>
			</div>
		</header>

		<section class="od__card" aria-label="Items">
			<h2 class="od__section-title">Items</h2>
			<div class="od__table-wrap">
				<table class="od__table">
					<thead>
						<tr>
							<th scope="col">Product</th>
							<th scope="col" class="od__col-sku">SKU</th>
							<th scope="col" class="od__col-num">Qty</th>
							<th scope="col" class="od__col-num">Unit price</th>
							<th scope="col" class="od__col-num">Line total</th>
						</tr>
					</thead>
					<tbody>
						{#each detail.items as item (item.id)}
							<tr>
								<th scope="row" class="od__cell-name">{item.name}</th>
								<td class="od__cell-sku">{item.sku ?? '—'}</td>
								<td class="od__col-num">{item.quantity}</td>
								<td class="od__col-num">
									{formatMoney(item.unit_price_cents, detail.order.currency)}
								</td>
								<td class="od__col-num">
									{formatMoney(item.line_total_cents, detail.order.currency)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<dl class="od__totals">
				<div>
					<dt>Subtotal</dt>
					<dd>{formatMoney(detail.order.subtotal_cents, detail.order.currency)}</dd>
				</div>
				{#if detail.order.discount_cents > 0}
					<div>
						<dt>Discount</dt>
						<dd>−{formatMoney(detail.order.discount_cents, detail.order.currency)}</dd>
					</div>
				{/if}
				{#if detail.order.tax_cents > 0}
					<div>
						<dt>Tax</dt>
						<dd>{formatMoney(detail.order.tax_cents, detail.order.currency)}</dd>
					</div>
				{/if}
				<div class="od__totals-grand">
					<dt>Total</dt>
					<dd>{formatMoney(detail.order.total_cents, detail.order.currency)}</dd>
				</div>
			</dl>
		</section>

		{#if detail.refunds.length > 0}
			<section class="od__card" aria-label="Refunds">
				<h2 class="od__section-title">Refunds</h2>
				<ul class="od__refunds" role="list">
					{#each detail.refunds as refund (refund.id)}
						<li class="od__refund">
							<div class="od__refund-amt">
								−{formatMoney(refund.amount_cents, detail.order.currency)}
							</div>
							<div class="od__refund-meta">
								<span>{formatDateTime(refund.created_at)}</span>
								{#if refund.reason}
									<span class="od__refund-reason">{refund.reason}</span>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		<section class="od__card" aria-label="History">
			<button
				type="button"
				class="od__history-toggle"
				onclick={() => (showHistory = !showHistory)}
				aria-expanded={showHistory}
				aria-controls="order-history"
			>
				{#if showHistory}
					<CaretDownIcon size={14} weight="bold" />
				{:else}
					<CaretRightIcon size={14} weight="bold" />
				{/if}
				<span>{showHistory ? 'Hide history' : 'Show history'}</span>
				<span class="od__history-count">({detail.state_log.length})</span>
			</button>
			{#if showHistory}
				<ol id="order-history" class="od__timeline">
					{#each detail.state_log as event, i (i)}
						<li class="od__event">
							<div class="od__event-dot" aria-hidden="true"></div>
							<div class="od__event-body">
								<div class="od__event-row">
									<span class="badge badge--grey"
										>{statusLabel(event.from_status)}</span
									>
									<CaretRightIcon size={12} weight="bold" />
									<span class="badge badge--{statusTone(event.to_status)}">
										{statusLabel(event.to_status)}
									</span>
									<span class="od__event-time"
										>{formatDateTime(event.created_at)}</span
									>
								</div>
								{#if event.reason}
									<p class="od__event-reason">{event.reason}</p>
								{/if}
							</div>
						</li>
					{/each}
				</ol>
			{/if}
		</section>
	{/if}
</section>

<style>
	.od {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.od__back {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
		align-self: flex-start;
	}

	.od__back:hover {
		text-decoration: underline;
	}

	.od__loading {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		padding: 1.5rem 0;
	}

	.od__error {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}

	.od__notfound {
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

	.od__header {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-top: 2px solid var(--color-teal);
		border-radius: var(--radius-xl);
		padding: 1.25rem 1.5rem;
	}

	.od__header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.od__num {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.od__placed {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		margin-top: 0.25rem;
	}

	.od__header-meta {
		display: inline-flex;
		align-items: center;
		gap: 1rem;
	}

	.od__total {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-variant-numeric: tabular-nums;
	}

	.od__card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.od__section-title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.od__table-wrap {
		overflow-x: auto;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.05);
	}

	.od__table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}

	.od__table thead {
		background-color: rgba(255, 255, 255, 0.02);
	}

	.od__table th,
	.od__table td {
		text-align: left;
		padding: 0.7rem 0.9rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		color: var(--color-grey-300);
	}

	.od__table tbody tr:last-child th,
	.od__table tbody tr:last-child td {
		border-bottom: none;
	}

	.od__table thead th {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-400);
	}

	.od__cell-name {
		color: var(--color-white);
		font-weight: var(--w-semibold);
	}

	.od__cell-sku {
		color: var(--color-grey-400);
		font-family: ui-monospace, SFMono-Regular, monospace;
		font-size: var(--fs-xs);
	}

	.od__col-sku {
		min-width: 6rem;
	}

	.od__col-num {
		text-align: right;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	.od__totals {
		margin: 0;
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.45rem;
		padding-top: 0.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.od__totals > div {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}

	.od__totals dt {
		color: var(--color-grey-400);
	}

	.od__totals dd {
		margin: 0;
		font-variant-numeric: tabular-nums;
		color: var(--color-grey-300);
	}

	.od__totals-grand {
		font-weight: var(--w-bold);
		padding-top: 0.4rem;
		margin-top: 0.25rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		font-size: var(--fs-md);
	}

	.od__totals-grand dt,
	.od__totals-grand dd {
		color: var(--color-white);
	}

	.od__refunds {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.od__refund {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.75rem 0.9rem;
		background-color: rgba(15, 164, 175, 0.06);
		border: 1px solid rgba(15, 164, 175, 0.18);
		border-radius: var(--radius-lg);
	}

	.od__refund-amt {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-teal);
		font-variant-numeric: tabular-nums;
	}

	.od__refund-meta {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-align: right;
	}

	.od__refund-reason {
		color: var(--color-grey-300);
	}

	.od__history-toggle {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		background: transparent;
		border: none;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		padding: 0;
		align-self: flex-start;
	}

	.od__history-count {
		color: var(--color-grey-400);
		font-weight: var(--w-medium);
	}

	.od__timeline {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.od__event {
		display: grid;
		grid-template-columns: 0.75rem 1fr;
		gap: 0.75rem;
		align-items: start;
	}

	.od__event-dot {
		width: 0.65rem;
		height: 0.65rem;
		margin-top: 0.4rem;
		background-color: var(--color-teal);
		border-radius: var(--radius-full);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.18);
	}

	.od__event-body {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.od__event-row {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		flex-wrap: wrap;
		color: var(--color-grey-400);
	}

	.od__event-time {
		margin-left: auto;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.od__event-reason {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
	}

	.badge {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.02em;
		white-space: nowrap;
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

	@media (max-width: 640px) {
		.od__header,
		.od__card {
			padding: 1rem;
		}

		.od__header-meta {
			width: 100%;
			justify-content: space-between;
		}
	}
</style>
