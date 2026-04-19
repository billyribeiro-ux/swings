<script lang="ts">
	import { onMount } from 'svelte';
	import Receipt from 'phosphor-svelte/lib/Receipt';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import DownloadSimple from 'phosphor-svelte/lib/DownloadSimple';
	import ArrowClockwise from 'phosphor-svelte/lib/ArrowClockwise';
	import Plus from 'phosphor-svelte/lib/Plus';
	import XCircle from 'phosphor-svelte/lib/XCircle';
	import Money from 'phosphor-svelte/lib/Money';
	import { auth } from '$lib/stores/auth.svelte';
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

	let envelope = $state<OrderListEnvelope | null>(null);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');
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

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

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
			flash(`Order ${detail.order.number} created`);
			showCreate = false;
			manualEmail = '';
			manualNotes = '';
			manualItems = [{ product_id: '', quantity: 1, unit_price_cents: 0, name: '' }];
			await refresh();
			selected = detail;
		} catch (e) {
			if (e instanceof ApiError) error = `${e.status}: ${e.message}`;
			else error = 'Manual order creation failed';
		} finally {
			createBusy = false;
		}
	}

	async function refund() {
		if (!selected) return;
		const amount = Number(refundAmount);
		if (!Number.isFinite(amount) || amount <= 0) {
			error = 'Refund amount must be > 0 cents';
			return;
		}
		refundBusy = true;
		error = '';
		try {
			await adminOrders.refund(selected.order.id, {
				amount_cents: Math.round(amount),
				reason: refundReason.trim() || undefined
			});
			flash(`Refunded ${formatMoney(amount, selected.order.currency)}`);
			await Promise.all([refresh(), inspect(selected.order)]);
			refundAmount = '';
			refundReason = '';
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Refund failed';
		} finally {
			refundBusy = false;
		}
	}

	async function voidOrder() {
		if (!selected) return;
		if (!confirm(`Void order ${selected.order.number}?`)) return;
		voidBusy = true;
		error = '';
		try {
			await adminOrders.void(selected.order.id, {
				reason: voidReason.trim() || undefined
			});
			flash('Order voided');
			await Promise.all([refresh(), inspect(selected.order)]);
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Void failed';
		} finally {
			voidBusy = false;
		}
	}

	async function downloadCsv() {
		try {
			const url = adminOrders.exportCsvUrl(buildQuery());
			const res = await fetch(url, {
				headers: { Authorization: auth.accessToken ? `Bearer ${auth.accessToken}` : '' }
			});
			if (!res.ok) {
				error = `Export failed (${res.status})`;
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
		} catch {
			error = 'CSV export failed';
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
		<div class="page__title-row">
			<Receipt size={28} weight="duotone" />
			<h1 class="page__title">Orders</h1>
		</div>
		<p class="page__subtitle">
			Search, inspect, void, and partially refund any digital-goods order. Manual order
			creation supports comp pricing, off-band Stripe refund ids, and FSM-aware bulk completion.
		</p>
	</header>

	{#if toast}<div class="toast">{toast}</div>{/if}
	{#if error}<div class="error" role="alert">{error}</div>{/if}

	<form class="filters" onsubmit={applyFilters}>
		<div class="filters__grid">
			<div class="field field--wide">
				<label class="field__label" for="ord-q">Search</label>
				<div class="search-input">
					<MagnifyingGlass size={16} />
					<input
						id="ord-q"
						class="field__input"
						placeholder="email or order number"
						bind:value={filters.q}
					/>
				</div>
			</div>
			<div class="field">
				<label class="field__label" for="ord-status">Status</label>
				<select id="ord-status" class="field__input" bind:value={filters.status}>
					<option value="">Any</option>
					<option value="pending">pending</option>
					<option value="completed">completed</option>
					<option value="refunded">refunded</option>
					<option value="cancelled">cancelled</option>
				</select>
			</div>
			<div class="field field--actions">
				<button class="btn btn--primary" type="submit">Apply</button>
				<button class="btn btn--ghost" type="button" onclick={refresh}>
					<ArrowClockwise size={16} weight="bold" />
				</button>
				<button class="btn btn--ghost" type="button" onclick={downloadCsv}>
					<DownloadSimple size={16} weight="bold" />
					CSV
				</button>
				<button
					class="btn btn--primary"
					type="button"
					onclick={() => (showCreate = !showCreate)}
				>
					<Plus size={16} weight="bold" />
					Manual order
				</button>
			</div>
		</div>
	</form>

	{#if showCreate}
		<section class="card create">
			<h2 class="card__title">Manual order</h2>
			<form class="create-form" onsubmit={createManual}>
				<div class="row-fields">
					<div class="field">
						<label class="field__label" for="m-email">Customer email</label>
						<input
							id="m-email"
							class="field__input"
							type="email"
							bind:value={manualEmail}
							required
						/>
					</div>
					<div class="field field--narrow">
						<label class="field__label" for="m-curr">Currency</label>
						<input
							id="m-curr"
							class="field__input"
							maxlength="3"
							bind:value={manualCurrency}
						/>
					</div>
					<label class="field field--checkbox">
						<input type="checkbox" bind:checked={manualMarkCompleted} />
						<span>Mark completed</span>
					</label>
				</div>
				<div class="items">
					{#each manualItems as _item, i (i)}
						<div class="item-row">
							<input
								class="field__input"
								placeholder="Product UUID"
								bind:value={manualItems[i].product_id}
								required
							/>
							<input
								class="field__input"
								placeholder="Display name"
								bind:value={manualItems[i].name}
								required
							/>
							<input
								class="field__input"
								placeholder="SKU (opt)"
								bind:value={manualItems[i].sku}
							/>
							<input
								class="field__input"
								type="number"
								min="1"
								placeholder="qty"
								bind:value={manualItems[i].quantity}
								required
							/>
							<input
								class="field__input"
								type="number"
								min="0"
								placeholder="unit cents"
								bind:value={manualItems[i].unit_price_cents}
								required
							/>
							<button
								type="button"
								class="btn btn--ghost btn--small"
								onclick={() => removeManualItem(i)}
								aria-label="Remove item"
							>
								<XCircle size={14} />
							</button>
						</div>
					{/each}
					<button type="button" class="btn btn--ghost btn--small" onclick={addManualItem}>
						<Plus size={14} />
						Add line
					</button>
				</div>
				<div class="field">
					<label class="field__label" for="m-notes">Notes</label>
					<textarea
						id="m-notes"
						class="field__input"
						rows="2"
						bind:value={manualNotes}
					></textarea>
				</div>
				<div class="form-actions">
					<button class="btn btn--ghost" type="button" onclick={() => (showCreate = false)}>
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
		<p class="muted">Loading…</p>
	{:else if !envelope || envelope.data.length === 0}
		<p class="muted">No orders match.</p>
	{:else}
		<div class="card table-wrap">
			<table class="table">
				<thead>
					<tr>
						<th>Number</th>
						<th>Email</th>
						<th>Status</th>
						<th>Total</th>
						<th>Placed</th>
						<th aria-label="Inspect"></th>
					</tr>
				</thead>
				<tbody>
					{#each envelope.data as o (o.id)}
						<tr>
							<td><code>{o.number}</code></td>
							<td>{o.email}</td>
							<td><span class="badge {statusClass(o.status)}">{o.status}</span></td>
							<td>{formatMoney(o.total_cents, o.currency)}</td>
							<td>
								{o.placed_at
									? new Date(o.placed_at).toLocaleString()
									: new Date(o.created_at).toLocaleString()}
							</td>
							<td>
								<button class="btn btn--ghost btn--small" onclick={() => inspect(o)}>
									Inspect
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<div class="pager">
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) <= 1}
				onclick={() => {
					filters.offset = Math.max(0, (filters.offset ?? 0) - (filters.limit ?? 25));
					void refresh();
				}}
			>
				Prev
			</button>
			<span class="pager__info">
				Page {envelope.page} / {envelope.total_pages || 1} · {envelope.total} orders
			</span>
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={() => {
					filters.offset = (filters.offset ?? 0) + (filters.limit ?? 25);
					void refresh();
				}}
			>
				Next
			</button>
		</div>
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
		<aside class="drawer">
			<header class="drawer__header">
				<h2 class="drawer__title">
					{selected.order.number}
					<span class="badge {statusClass(selected.order.status)}">
						{selected.order.status}
					</span>
				</h2>
				<button class="btn btn--ghost btn--small" onclick={() => (selected = null)}>
					Close
				</button>
			</header>
			<dl class="drawer__meta">
				<dt>Customer</dt>
				<dd>{selected.order.email}</dd>
				<dt>Total</dt>
				<dd>{formatMoney(selected.order.total_cents, selected.order.currency)}</dd>
				<dt>Refunded</dt>
				<dd>{formatMoney(selected.refunded_cents, selected.order.currency)}</dd>
				<dt>Remaining refundable</dt>
				<dd>
					{formatMoney(selected.remaining_refundable_cents, selected.order.currency)}
				</dd>
				{#if selected.order.stripe_payment_intent_id}
					<dt>Stripe PI</dt>
					<dd><code>{selected.order.stripe_payment_intent_id}</code></dd>
				{/if}
			</dl>

			<h3 class="drawer__section">Items</h3>
			<table class="mini-table">
				<thead>
					<tr><th>Name</th><th>Qty</th><th>Unit</th><th>Line</th></tr>
				</thead>
				<tbody>
					{#each selected.items as it (it.id)}
						<tr>
							<td>{it.name}</td>
							<td>{it.quantity}</td>
							<td>{formatMoney(it.unit_price_cents, selected.order.currency)}</td>
							<td>{formatMoney(it.line_total_cents, selected.order.currency)}</td>
						</tr>
					{/each}
				</tbody>
			</table>

			{#if selected.refunds.length > 0}
				<h3 class="drawer__section">Refunds</h3>
				<table class="mini-table">
					<thead>
						<tr><th>Amount</th><th>Reason</th><th>When</th></tr>
					</thead>
					<tbody>
						{#each selected.refunds as r (r.id)}
							<tr>
								<td>{formatMoney(r.amount_cents, selected.order.currency)}</td>
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
						<Money size={18} weight="duotone" />
						Refund
					</h3>
					<div class="row-fields">
						<input
							class="field__input"
							type="number"
							min="1"
							max={selected.remaining_refundable_cents}
							placeholder="amount cents"
							bind:value={refundAmount}
						/>
						<input
							class="field__input"
							placeholder="reason (optional)"
							bind:value={refundReason}
						/>
						<button
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
						<XCircle size={18} weight="duotone" />
						Void
					</h3>
					<div class="row-fields">
						<input
							class="field__input"
							placeholder="reason (optional)"
							bind:value={voidReason}
						/>
						<button class="btn btn--danger" onclick={voidOrder} disabled={voidBusy}>
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
		max-width: 1280px;
	}
	.page__header {
		margin-bottom: var(--space-5);
	}
	.page__title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-white);
	}
	.page__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		margin: 0;
	}
	.page__subtitle {
		margin-top: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		max-width: 75ch;
	}
	.toast {
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.filters {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-4);
		margin-bottom: var(--space-4);
	}
	.filters__grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: var(--space-3);
		align-items: end;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field--wide {
		grid-column: span 2;
	}
	.field--narrow {
		max-width: 8rem;
	}
	.field--actions {
		flex-direction: row;
		gap: var(--space-2);
		align-items: end;
	}
	.field--checkbox {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}
	.field__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
	}
	.field__input {
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		width: 100%;
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.search-input {
		position: relative;
	}
	.search-input :global(svg) {
		position: absolute;
		left: 0.7rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-400);
		pointer-events: none;
	}
	.search-input .field__input {
		padding-left: 2rem;
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
	}
	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.btn--primary {
		background: var(--color-teal);
		color: var(--color-white);
	}
	.btn--ghost {
		border-color: rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
	}
	.btn--ghost:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}
	.btn--danger {
		background: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
		border-color: rgba(239, 68, 68, 0.25);
	}
	.btn--small {
		padding: 0.25rem 0.6rem;
		font-size: var(--fs-xs);
	}
	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-5);
		margin-bottom: var(--space-4);
	}
	.card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0 0 var(--space-3) 0;
	}
	.create-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.row-fields {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
		align-items: end;
	}
	.row-fields > .field {
		flex: 1 1 200px;
	}
	.items {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		background: rgba(0, 0, 0, 0.15);
		border-radius: var(--radius-lg);
	}
	.item-row {
		display: grid;
		grid-template-columns: 1.4fr 1.2fr 0.8fr 0.5fr 0.7fr auto;
		gap: var(--space-2);
		align-items: center;
	}
	.form-actions {
		display: flex;
		gap: var(--space-2);
		justify-content: flex-end;
	}
	.table-wrap {
		overflow-x: auto;
		padding: var(--space-3);
	}
	.table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.table th {
		text-align: left;
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		padding: var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		white-space: nowrap;
	}
	.table td {
		padding: var(--space-3) var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}
	.badge {
		display: inline-block;
		padding: 0.1rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.badge--ok {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}
	.badge--warn {
		background: rgba(245, 158, 11, 0.15);
		color: #fbbf24;
	}
	.badge--off {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
	.badge--err {
		background: rgba(239, 68, 68, 0.15);
		color: #fca5a5;
	}
	.pager {
		display: flex;
		gap: var(--space-3);
		justify-content: center;
		align-items: center;
		margin-top: var(--space-4);
	}
	.pager__info {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}
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
		padding: var(--space-5);
		overflow-y: auto;
		z-index: 70;
		box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3);
	}
	.drawer__header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}
	.drawer__title {
		font-size: var(--fs-lg);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.drawer__meta {
		display: grid;
		grid-template-columns: 9rem 1fr;
		gap: var(--space-2) var(--space-3);
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
		margin-bottom: var(--space-4);
	}
	.drawer__meta dt {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.drawer__meta dd {
		margin: 0;
		word-break: break-all;
	}
	.drawer__section {
		font-size: var(--fs-sm);
		color: var(--color-white);
		margin: var(--space-4) 0 var(--space-2);
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.mini-table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-xs);
	}
	.mini-table th {
		text-align: left;
		color: var(--color-grey-400);
		padding: 0.4rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.mini-table td {
		padding: 0.5rem 0.4rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}
	.action-card {
		margin-top: var(--space-4);
		padding: var(--space-4);
		background: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.06);
	}
	.action-card--danger {
		border-color: rgba(239, 68, 68, 0.2);
	}
</style>
