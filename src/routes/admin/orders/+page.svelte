<script lang="ts">
	import { onMount } from 'svelte';
	import ReceiptIcon from 'phosphor-svelte/lib/ReceiptIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import DownloadSimpleIcon from 'phosphor-svelte/lib/DownloadSimpleIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import MoneyIcon from 'phosphor-svelte/lib/MoneyIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';
	import { ApiError } from '$lib/api/client';
	import {
		adminOrders,
		formatMoney,
		type Order,
		type OrderDetail,
		type OrderListEnvelope,
		type OrderListQuery,
		type ManualOrderItemInput
	} from '$lib/api/admin-orders';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import { toast } from '$lib/stores/toast.svelte';

	let envelope = $state<OrderListEnvelope | null>(null);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<OrderDetail | null>(null);
	let showCreate = $state(false);

	let filters = $state<OrderListQuery>({ q: '', status: '', limit: 25, offset: 0 });

	let createBusy = $state(false);
	let manualEmail = $state('');
	let manualCurrency = $state('usd');
	let manualMarkCompleted = $state(false);
	let manualNotes = $state('');
	let manualItems = $state<ManualOrderItemInput[]>([
		{ product_id: '', quantity: 1, unit_price_cents: 0, name: '' }
	]);

	let refundAmount = $state('');
	let refundReason = $state('');
	let refundBusy = $state(false);

	let voidReason = $state('');
	let voidBusy = $state(false);

	function buildQuery(): OrderListQuery {
		const q: OrderListQuery = { limit: filters.limit ?? 25, offset: filters.offset ?? 0 };
		if (filters.q?.trim()) q.q = filters.q.trim();
		if (filters.status?.trim()) q.status = filters.status.trim();
		return q;
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await adminOrders.list(buildQuery());
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load orders';
		} finally {
			loading = false;
		}
	}

	function applyFilters(e: Event) {
		e.preventDefault();
		filters.offset = 0;
		void refresh();
	}

	async function inspect(o: Order) {
		try {
			selected = await adminOrders.get(o.id);
			refundAmount = '';
			refundReason = '';
			voidReason = '';
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to fetch order';
		}
	}

	function addManualItem() {
		manualItems = [
			...manualItems,
			{ product_id: '', quantity: 1, unit_price_cents: 0, name: '' }
		];
	}

	function removeManualItem(i: number) {
		manualItems = manualItems.filter((_, idx) => idx !== i);
		if (manualItems.length === 0) addManualItem();
	}

	async function createManual(e: Event) {
		e.preventDefault();
		createBusy = true;
		error = '';
		try {
			const detail = await adminOrders.createManual({
				email: manualEmail.trim(),
				currency: manualCurrency.trim().toLowerCase() || 'usd',
				items: manualItems.map((it) => ({
					product_id: it.product_id.trim(),
					quantity: Number(it.quantity),
					unit_price_cents: Number(it.unit_price_cents),
					name: it.name.trim(),
					sku: it.sku?.trim() || undefined
				})),
				mark_completed: manualMarkCompleted,
				notes: manualNotes.trim() || undefined
			});
			toast.success(`Order ${detail.order.number} created`);
			showCreate = false;
			manualEmail = '';
			manualNotes = '';
			manualItems = [{ product_id: '', quantity: 1, unit_price_cents: 0, name: '' }];
			await refresh();
			selected = detail;
		} catch (e) {
			const description = e instanceof ApiError ? `${e.status}: ${e.message}` : undefined;
			toast.error('Manual order creation failed', { description });
		} finally {
			createBusy = false;
		}
	}

	async function refund() {
		if (!selected) return;
		const amount = Number(refundAmount);
		if (!Number.isFinite(amount) || amount <= 0) {
			toast.warning('Refund amount must be greater than 0 cents');
			return;
		}
		refundBusy = true;
		error = '';
		try {
			await adminOrders.refund(selected.order.id, {
				amount_cents: Math.round(amount),
				reason: refundReason.trim() || undefined
			});
			toast.success(`Refunded ${formatMoney(amount, selected.order.currency)}`);
			await Promise.all([refresh(), inspect(selected.order)]);
			refundAmount = '';
			refundReason = '';
		} catch (e) {
			toast.error('Refund failed', {
				description: e instanceof ApiError ? `${e.status}: ${e.message}` : undefined
			});
		} finally {
			refundBusy = false;
		}
	}

	async function voidOrder() {
		if (!selected) return;
		const ok = await confirmDialog({
			title: `Void order ${selected.order.number}?`,
			message:
				'Voiding cancels the order and releases any reserved inventory. This cannot be undone.',
			confirmLabel: 'Void order',
			variant: 'danger'
		});
		if (!ok) return;
		voidBusy = true;
		error = '';
		try {
			await adminOrders.void(selected.order.id, {
				reason: voidReason.trim() || undefined
			});
			toast.success('Order voided');
			await Promise.all([refresh(), inspect(selected.order)]);
		} catch (e) {
			toast.error('Void failed', {
				description: e instanceof ApiError ? `${e.status}: ${e.message}` : undefined
			});
		} finally {
			voidBusy = false;
		}
	}

	async function downloadCsv() {
		try {
			const url = adminOrders.exportCsvUrl(buildQuery());
			// BFF (Phase 1.3): cookie-based auth — no Bearer header needed.
			const res = await fetch(url, { credentials: 'include' });
			if (!res.ok) {
				toast.error('Orders export failed', {
					description: `Server returned ${res.status}`
				});
				return;
			}
			const blob = await res.blob();
			const a = document.createElement('a');
			const u = URL.createObjectURL(blob);
			a.href = u;
			a.download = `orders-${new Date().toISOString().slice(0, 10)}.csv`;
			document.body.appendChild(a);
			a.click();
			a.remove();
			URL.revokeObjectURL(u);
			toast.success('Orders CSV exported');
		} catch (e) {
			toast.error('Orders export failed', {
				description: e instanceof Error ? e.message : undefined
			});
		}
	}

	function statusClass(s: string): string {
		switch (s) {
			case 'completed':
				return 'badge--ok';
			case 'pending':
				return 'badge--warn';
			case 'cancelled':
				return 'badge--off';
			case 'refunded':
				return 'badge--err';
			default:
				return 'badge--off';
		}
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Orders · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-orders-page">
	<header class="page__header">
		<div class="page__heading">
			<span class="page__eyebrow">Commerce</span>
			<h1 class="page__title">Orders</h1>
			<p class="page__subtitle">
				Search, inspect, void, and partially refund any digital-goods order. Manual order
				creation supports comp pricing and FSM-aware bulk completion.
			</p>
		</div>
	</header>

	{#if error}<div class="error" role="alert">{error}</div>{/if}

	<form class="filter-card" onsubmit={applyFilters}>
		<div class="filter-card__field filter-card__field--search">
			<label class="filter-card__label" for="ord-q">Search</label>
			<div class="search-wrap">
				<MagnifyingGlassIcon size={16} weight="bold" class="search-icon" />
				<input
					id="ord-q"
					name="ord-q"
					type="search"
					class="filter-input filter-input--search"
					placeholder="Email or order number"
					bind:value={filters.q}
				/>
			</div>
		</div>
		<div class="filter-card__field">
			<label class="filter-card__label" for="ord-status">Status</label>
			<div class="select-wrap">
				<select
					id="ord-status"
					name="ord-status"
					class="filter-input filter-input--select"
					bind:value={filters.status}
				>
					<option value="">Any</option>
					<option value="pending">Pending</option>
					<option value="completed">Completed</option>
					<option value="refunded">Refunded</option>
					<option value="cancelled">Cancelled</option>
				</select>
				<CaretDownIcon size={14} weight="bold" class="select-caret" />
			</div>
		</div>
		<div class="filter-card__actions">
			<button class="btn btn--primary" type="submit">
				<MagnifyingGlassIcon size={16} weight="bold" />
				<span>Apply</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={refresh} title="Refresh">
				<ArrowsClockwiseIcon size={16} weight="bold" />
				<span class="filter-card__actions-label">Refresh</span>
			</button>
			<button
				class="btn btn--secondary"
				type="button"
				onclick={downloadCsv}
				title="Export CSV"
			>
				<DownloadSimpleIcon size={16} weight="bold" />
				<span class="filter-card__actions-label">Export</span>
			</button>
			<button
				class="btn btn--primary"
				type="button"
				onclick={() => (showCreate = !showCreate)}
			>
				<PlusIcon size={16} weight="bold" />
				<span>Manual order</span>
			</button>
		</div>
	</form>

	{#if showCreate}
		<section class="card create">
			<h2 class="card__title">Manual order</h2>
			<form class="create-form" onsubmit={createManual}>
				<div class="row-fields">
					<div class="cf-field">
						<label class="cf-label" for="m-email">Customer email</label>
						<input
							id="m-email"
							name="m-email"
							class="cf-input"
							type="email"
							bind:value={manualEmail}
							required
						/>
					</div>
					<div class="cf-field cf-field--narrow">
						<label class="cf-label" for="m-curr">Currency</label>
						<input
							id="m-curr"
							name="m-curr"
							class="cf-input"
							maxlength="3"
							bind:value={manualCurrency}
						/>
					</div>
					<label class="cf-checkbox">
						<input
							id="m-completed"
							name="m-completed"
							type="checkbox"
							bind:checked={manualMarkCompleted}
						/>
						<span>Mark completed</span>
					</label>
				</div>
				<div class="items">
					{#each manualItems as _item, i (i)}
						<div class="item-row">
							<input
								class="cf-input"
								placeholder="Product UUID"
								bind:value={manualItems[i].product_id}
								aria-label="Product UUID"
								required
							/>
							<input
								class="cf-input"
								placeholder="Display name"
								bind:value={manualItems[i].name}
								aria-label="Display name"
								required
							/>
							<input
								class="cf-input"
								placeholder="SKU (opt)"
								bind:value={manualItems[i].sku}
								aria-label="SKU"
							/>
							<input
								class="cf-input"
								type="number"
								min="1"
								placeholder="Qty"
								bind:value={manualItems[i].quantity}
								aria-label="Quantity"
								required
							/>
							<input
								class="cf-input"
								type="number"
								min="0"
								placeholder="Unit cents"
								bind:value={manualItems[i].unit_price_cents}
								aria-label="Unit price (cents)"
								required
							/>
							<Tooltip label="Remove item">
								<button
									type="button"
									class="icon-btn icon-btn--danger"
									onclick={() => removeManualItem(i)}
									aria-label="Remove item"
								>
									<XCircleIcon size={16} weight="bold" />
								</button>
							</Tooltip>
						</div>
					{/each}
					<button
						type="button"
						class="btn btn--secondary btn--small"
						onclick={addManualItem}
					>
						<PlusIcon size={14} weight="bold" />
						<span>Add line</span>
					</button>
				</div>
				<div class="cf-field">
					<label class="cf-label" for="m-notes">Notes</label>
					<textarea
						id="m-notes"
						name="m-notes"
						class="cf-input"
						rows="2"
						bind:value={manualNotes}
					></textarea>
				</div>
				<div class="form-actions">
					<button
						class="btn btn--secondary"
						type="button"
						onclick={() => (showCreate = false)}
					>
						Cancel
					</button>
					<button class="btn btn--primary" type="submit" disabled={createBusy}>
						{createBusy ? 'Creating…' : 'Create order'}
					</button>
				</div>
			</form>
		</section>
	{/if}

	{#if loading}
		<div class="muted card card--center">Loading…</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty-state">
			<ReceiptIcon size={48} weight="duotone" />
			<h2 class="empty-state__title">No orders match</h2>
			<p class="empty-state__desc">
				Try widening your search, clearing the status filter, or creating a manual order.
			</p>
		</div>
	{:else}
		<div class="card table-wrap">
			<table class="table">
				<thead>
					<tr>
						<th>Number</th>
						<th>Email</th>
						<th>Status</th>
						<th class="table__num">Total</th>
						<th>Placed</th>
						<th class="table__actions-h" aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each envelope.data as o (o.id)}
						<tr>
							<td><code class="mono">{o.number}</code></td>
							<td class="table__email">{o.email}</td>
							<td><span class="badge {statusClass(o.status)}">{o.status}</span></td>
							<td class="table__num">{formatMoney(o.total_cents, o.currency)}</td>
							<td class="table__date">
								{o.placed_at
									? new Date(o.placed_at).toLocaleString()
									: new Date(o.created_at).toLocaleString()}
							</td>
							<td class="table__actions">
								<Tooltip label="Inspect order {o.number}">
									<button
										type="button"
										class="icon-btn"
										onclick={() => inspect(o)}
										aria-label="Inspect order {o.number}"
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
		<nav class="pager" aria-label="Pagination">
			<button
				type="button"
				class="page-btn"
				disabled={(envelope.page ?? 1) <= 1}
				onclick={() => {
					filters.offset = Math.max(0, (filters.offset ?? 0) - (filters.limit ?? 25));
					void refresh();
				}}
			>
				<CaretLeftIcon size={14} weight="bold" />
				<span>Previous</span>
			</button>
			<span class="page-info">
				Page {envelope.page} of {envelope.total_pages || 1} · {envelope.total} orders
			</span>
			<button
				type="button"
				class="page-btn"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={() => {
					filters.offset = (filters.offset ?? 0) + (filters.limit ?? 25);
					void refresh();
				}}
			>
				<span>Next</span>
				<CaretRightIcon size={14} weight="bold" />
			</button>
		</nav>
	{/if}

	{#if selected}
		<div
			class="drawer-backdrop"
			role="button"
			tabindex="-1"
			aria-label="Close"
			onclick={() => (selected = null)}
			onkeydown={(e) => e.key === 'Escape' && (selected = null)}
		></div>
		<aside class="drawer" aria-label="Order details">
			<header class="drawer__header">
				<h2 class="drawer__title">
					{selected.order.number}
					<span class="badge {statusClass(selected.order.status)}">
						{selected.order.status}
					</span>
				</h2>
				<Tooltip label="Close" placement="left">
					<button
						type="button"
						class="icon-btn"
						onclick={() => (selected = null)}
						aria-label="Close drawer"
					>
						<XIcon size={16} weight="bold" />
					</button>
				</Tooltip>
			</header>
			<dl class="drawer__meta">
				<dt>Customer</dt>
				<dd>{selected.order.email}</dd>
				<dt>Total</dt>
				<dd class="num">
					{formatMoney(selected.order.total_cents, selected.order.currency)}
				</dd>
				<dt>Refunded</dt>
				<dd class="num">{formatMoney(selected.refunded_cents, selected.order.currency)}</dd>
				<dt>Remaining refundable</dt>
				<dd class="num">
					{formatMoney(selected.remaining_refundable_cents, selected.order.currency)}
				</dd>
				{#if selected.order.stripe_payment_intent_id}
					<dt>Stripe PI</dt>
					<dd><code class="mono">{selected.order.stripe_payment_intent_id}</code></dd>
				{/if}
			</dl>

			<h3 class="drawer__section">Items</h3>
			<table class="mini-table">
				<thead>
					<tr
						><th>Name</th><th class="mini-table__num">Qty</th><th
							class="mini-table__num">Unit</th
						><th class="mini-table__num">Line</th></tr
					>
				</thead>
				<tbody>
					{#each selected.items as it (it.id)}
						<tr>
							<td>{it.name}</td>
							<td class="mini-table__num">{it.quantity}</td>
							<td class="mini-table__num"
								>{formatMoney(it.unit_price_cents, selected.order.currency)}</td
							>
							<td class="mini-table__num"
								>{formatMoney(it.line_total_cents, selected.order.currency)}</td
							>
						</tr>
					{/each}
				</tbody>
			</table>

			{#if selected.refunds.length > 0}
				<h3 class="drawer__section">Refunds</h3>
				<table class="mini-table">
					<thead>
						<tr><th class="mini-table__num">Amount</th><th>Reason</th><th>When</th></tr>
					</thead>
					<tbody>
						{#each selected.refunds as r (r.id)}
							<tr>
								<td class="mini-table__num"
									>{formatMoney(r.amount_cents, selected.order.currency)}</td
								>
								<td>{r.reason ?? '—'}</td>
								<td>{new Date(r.created_at).toLocaleString()}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}

			{#if selected.remaining_refundable_cents > 0 && selected.order.status !== 'cancelled'}
				<section class="action-card">
					<h3 class="drawer__section">
						<MoneyIcon size={18} weight="duotone" />
						Refund
					</h3>
					<div class="row-fields">
						<input
							class="cf-input"
							type="number"
							min="1"
							max={selected.remaining_refundable_cents}
							placeholder="Amount cents"
							bind:value={refundAmount}
							aria-label="Refund amount in cents"
						/>
						<input
							class="cf-input"
							placeholder="Reason (optional)"
							bind:value={refundReason}
							aria-label="Refund reason"
						/>
						<button
							type="button"
							class="btn btn--primary"
							onclick={refund}
							disabled={refundBusy}
						>
							{refundBusy ? 'Refunding…' : 'Refund'}
						</button>
					</div>
				</section>
			{/if}

			{#if selected.order.status !== 'cancelled' && selected.order.status !== 'refunded'}
				<section class="action-card action-card--danger">
					<h3 class="drawer__section">
						<XCircleIcon size={18} weight="duotone" />
						Void
					</h3>
					<div class="row-fields">
						<input
							class="cf-input"
							placeholder="Reason (optional)"
							bind:value={voidReason}
							aria-label="Void reason"
						/>
						<button
							type="button"
							class="btn btn--danger"
							onclick={voidOrder}
							disabled={voidBusy}
						>
							{voidBusy ? 'Voiding…' : 'Void order'}
						</button>
					</div>
				</section>
			{/if}
		</aside>
	{/if}
</div>

<style>
	.page {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		max-width: 1280px;
	}

	/* ── Header ─────────────────────────── */
	.page__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.page__heading {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}

	.page__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.page__title {
		font-size: 1.5rem;
		font-weight: 700;
		font-family: var(--font-heading);
		color: var(--color-white);
		line-height: 1.2;
		letter-spacing: -0.01em;
		margin: 0;
	}

	.page__subtitle {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
	}

	/* ── Status messages ────────────────── */
	.error {
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
	}

	.muted {
		color: var(--color-grey-400);
		font-size: 0.875rem;
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

	.filter-card__actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.filter-card__actions-label {
		display: none;
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
		border: 1px solid transparent;
		cursor: pointer;
		text-decoration: none;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			transform 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out),
			color 150ms var(--ease-out);
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
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

	.btn--danger {
		color: #fca5a5;
		background-color: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.3);
	}

	.btn--danger:hover:not(:disabled) {
		background-color: rgba(239, 68, 68, 0.18);
	}

	.btn--small {
		min-height: 2.5rem;
		padding: 0.4rem 0.75rem;
		font-size: 0.75rem;
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

	.icon-btn--danger:hover {
		background-color: rgba(239, 68, 68, 0.12);
		border-color: rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}

	/* ── Card / create form ─────────────── */
	.card {
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

	.card--center {
		text-align: center;
		padding: 2rem 1rem;
	}

	.card__title {
		font-size: 1rem;
		font-weight: 600;
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0 0 1rem 0;
	}

	.create-form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.row-fields {
		display: flex;
		gap: 0.75rem;
		flex-wrap: wrap;
		align-items: flex-end;
	}

	.row-fields > .cf-field {
		flex: 1 1 200px;
	}

	.cf-field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.cf-field--narrow {
		max-width: 8rem;
	}

	.cf-label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.cf-input {
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

	.cf-input::placeholder {
		color: var(--color-grey-500);
	}

	.cf-input:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.cf-checkbox {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: var(--color-grey-300);
		cursor: pointer;
		min-height: 3rem;
	}

	.items {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem;
		background: rgba(0, 0, 0, 0.15);
		border-radius: var(--radius-2xl);
	}

	.item-row {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.5rem;
		align-items: center;
	}

	.form-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	/* ── Empty state ────────────────────── */
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

	/* ── Table ──────────────────────────── */
	.table-wrap {
		overflow-x: auto;
		padding: 0;
	}

	.table {
		width: 100%;
		border-collapse: collapse;
		min-width: 720px;
		font-size: 0.875rem;
	}

	.table thead {
		background-color: rgba(255, 255, 255, 0.02);
	}

	.table th {
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

	.table td {
		padding: 0.875rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-300);
		line-height: 1.45;
	}

	.table tbody tr {
		transition: background-color 150ms var(--ease-out);
	}

	.table tbody tr:hover {
		background-color: rgba(255, 255, 255, 0.02);
	}

	.table tbody tr:last-child td {
		border-bottom: none;
	}

	.mono {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--color-white);
	}

	.table__email {
		color: var(--color-grey-300);
		max-width: 18rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.table__num {
		text-align: right;
		font-variant-numeric: tabular-nums;
		font-weight: 500;
		color: var(--color-white);
		white-space: nowrap;
	}

	.table__date {
		font-size: 0.75rem;
		color: var(--color-grey-400);
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}

	.table__actions-h,
	.table__actions {
		width: 3rem;
		text-align: right;
	}

	/* ── Badges ─────────────────────────── */
	.badge {
		display: inline-flex;
		align-items: center;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.badge--ok {
		background: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}

	.badge--warn {
		background: rgba(245, 158, 11, 0.12);
		color: #fcd34d;
	}

	.badge--off {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	.badge--err {
		background: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
	}

	/* ── Pagination ─────────────────────── */
	.pager {
		display: flex;
		gap: 0.75rem;
		justify-content: center;
		align-items: center;
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

	/* ── Drawer ─────────────────────────── */
	.drawer-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		z-index: 60;
	}

	.drawer {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		width: min(640px, 96vw);
		background: var(--color-navy);
		border-left: 1px solid rgba(255, 255, 255, 0.08);
		padding: 1.5rem;
		overflow-y: auto;
		z-index: 70;
		box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3);
	}

	.drawer__header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.drawer__title {
		font-size: 1rem;
		font-weight: 600;
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
		display: flex;
		gap: 0.5rem;
		align-items: center;
		letter-spacing: -0.01em;
	}

	.drawer__meta {
		display: grid;
		grid-template-columns: 9rem 1fr;
		gap: 0.5rem 0.75rem;
		font-size: 0.875rem;
		color: var(--color-grey-300);
		margin-bottom: 1rem;
	}

	.drawer__meta dt {
		color: var(--color-grey-500);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.drawer__meta dd {
		margin: 0;
		word-break: break-all;
		color: var(--color-grey-200);
	}

	.drawer__meta dd.num {
		font-variant-numeric: tabular-nums;
		color: var(--color-white);
		font-weight: 500;
	}

	.drawer__section {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 1rem 0 0.5rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.mini-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.75rem;
	}

	.mini-table th {
		text-align: left;
		color: var(--color-grey-500);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.5rem 0.4rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.mini-table td {
		padding: 0.5rem 0.4rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}

	.mini-table__num {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	.action-card {
		margin-top: 1rem;
		padding: 1rem;
		background: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.06);
	}

	.action-card--danger {
		border-color: rgba(239, 68, 68, 0.2);
	}

	/* ── Tablet 480px+ ──────────────────── */
	@media (min-width: 480px) {
		.item-row {
			grid-template-columns: 1.4fr 1.2fr 0.8fr 0.5fr 0.7fr auto;
		}
	}

	/* ── Tablet 768px+ ──────────────────── */
	@media (min-width: 768px) {
		.page {
			gap: 1.5rem;
		}

		.filter-card {
			flex-direction: row;
			flex-wrap: wrap;
			align-items: flex-end;
			padding: 1.5rem;
		}

		.filter-card__field--search {
			flex: 1 1 18rem;
		}

		.filter-card__field {
			flex: 0 0 11rem;
		}

		.filter-card__actions {
			margin-left: auto;
		}

		.filter-card__actions-label {
			display: inline;
		}

		.card {
			padding: 1.75rem;
		}
	}
</style>
