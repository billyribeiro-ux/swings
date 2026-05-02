<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import TagIcon from 'phosphor-svelte/lib/TagIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';

	interface CouponRedemption {
		id: string;
		coupon_code: string;
		discount_applied_cents: number;
		redeemed_at: string;
		subscription_id?: string | null;
		// Migration 085 added these so the history table can render the
		// discount with the right currency symbol and deep-link to the
		// order whose invoice carried the discount.
		currency: string;
		order_id?: string | null;
	}

	interface ApplyResponse {
		ok: boolean;
		coupon_id: string;
		applied_at: string;
		valid: boolean;
		message: string;
	}

	let redemptions = $state<CouponRedemption[]>([]);
	let loading = $state(true);
	let loadError = $state('');

	let code = $state('');
	let applyBusy = $state(false);
	let applyMessage = $state('');
	let applyError = $state('');

	function formatMoney(cents: number, currency = 'usd'): string {
		try {
			return new Intl.NumberFormat(undefined, {
				style: 'currency',
				currency: currency.toUpperCase()
			}).format(cents / 100);
		} catch {
			return `$${(cents / 100).toFixed(2)}`;
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

	async function loadHistory() {
		loading = true;
		loadError = '';
		try {
			redemptions = await api.get<CouponRedemption[]>('/api/member/coupons/redeemed');
		} catch (e) {
			loadError = e instanceof ApiError ? e.message : 'Failed to load coupons';
			redemptions = [];
		} finally {
			loading = false;
		}
	}

	async function handleApply(e: SubmitEvent) {
		e.preventDefault();
		const trimmed = code.trim();
		if (!trimmed) {
			applyError = 'Please enter a coupon code.';
			applyMessage = '';
			return;
		}
		applyBusy = true;
		applyMessage = '';
		applyError = '';
		try {
			const res = await api.post<ApplyResponse>('/api/member/coupons/apply', {
				code: trimmed
			});
			applyMessage = res.message || `Coupon ${trimmed} applied.`;
			code = '';
			await loadHistory();
		} catch (err) {
			applyError = err instanceof ApiError ? err.message : 'Failed to apply coupon';
		} finally {
			applyBusy = false;
		}
	}

	onMount(() => {
		void loadHistory();
	});
</script>

<svelte:head><title>Coupons - Precision Options Signals</title></svelte:head>

<section class="cp">
	<header class="cp__header">
		<h1 class="cp__title">Coupons</h1>
		<p class="cp__sub">Apply a promotional code or browse your redemption history.</p>
	</header>

	<section class="cp__card" aria-labelledby="apply-title">
		<h2 id="apply-title" class="cp__section-title">Apply a coupon</h2>
		<form class="cp__form" onsubmit={handleApply} novalidate>
			<label class="cp__label" for="coupon-code">Coupon code</label>
			<div class="cp__row">
				<input
					id="coupon-code"
					class="cp__input"
					type="text"
					name="code"
					autocomplete="off"
					autocapitalize="characters"
					spellcheck="false"
					placeholder="e.g. WELCOME20"
					bind:value={code}
					disabled={applyBusy}
				/>
				<button type="submit" class="btn btn--primary" disabled={applyBusy || !code.trim()}>
					{applyBusy ? 'Applying…' : 'Apply'}
				</button>
			</div>
			{#if applyMessage}
				<p class="cp__success" role="status">
					<CheckCircleIcon size={14} weight="fill" />
					{applyMessage}
				</p>
			{/if}
			{#if applyError}
				<p class="cp__error" role="alert">
					<XCircleIcon size={14} weight="fill" />
					{applyError}
				</p>
			{/if}
		</form>
	</section>

	<section class="cp__card" aria-labelledby="history-title">
		<h2 id="history-title" class="cp__section-title">Redemption history</h2>

		{#if loading}
			<p class="cp__muted">Loading history…</p>
		{:else if loadError}
			<div class="cp__error-block" role="alert">{loadError}</div>
		{:else if redemptions.length === 0}
			<div class="empty">
				<div class="empty__icon"><TagIcon size={32} weight="duotone" /></div>
				<h3 class="empty__title">No coupons used yet</h3>
				<p class="empty__body">When you redeem a coupon it will be listed here.</p>
			</div>
		{:else}
			<div class="cp__table-wrap">
				<table class="cp__table">
					<thead>
						<tr>
							<th scope="col">Code</th>
							<th scope="col" class="cp__col-num">Discount applied</th>
							<th scope="col">Date used</th>
							<th scope="col">Subscription</th>
							<th scope="col">Order</th>
						</tr>
					</thead>
					<tbody>
						{#each redemptions as r (r.id)}
							<tr>
								<th scope="row" class="cp__cell-code">{r.coupon_code}</th>
								<td class="cp__col-num">
									{r.discount_applied_cents > 0
										? `−${formatMoney(r.discount_applied_cents, r.currency)} ${r.currency.toUpperCase()}`
										: '—'}
								</td>
								<td class="cp__cell-date">{formatDate(r.redeemed_at)}</td>
								<td>
									{#if r.subscription_id}
										<a
											class="cp__link"
											href={resolve('/dashboard/account/subscriptions/[id]', {
												id: r.subscription_id
											})}
										>
											#{r.subscription_id.slice(0, 8)}
										</a>
									{:else}
										<span class="cp__dash">—</span>
									{/if}
								</td>
								<td>
									{#if r.order_id}
										<a
											class="cp__link"
											href={resolve('/dashboard/account/orders/[id]', {
												id: r.order_id
											})}
										>
											View order →
										</a>
									{:else}
										<span class="cp__dash">—</span>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</section>
</section>

<style>
	.cp {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.cp__header {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.cp__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.cp__sub {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.cp__card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.cp__section-title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.cp__muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.cp__form {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}
	.cp__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.cp__row {
		display: flex;
		gap: 0.65rem;
		align-items: stretch;
		flex-wrap: wrap;
	}
	.cp__input {
		flex: 1;
		min-width: 12rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: ui-monospace, SFMono-Regular, monospace;
		letter-spacing: 0.05em;
	}
	.cp__input:focus {
		outline: none;
		border-color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.06);
	}
	.cp__input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.65rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		border: 1px solid transparent;
		transition: opacity 200ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
	}
	.btn--primary:not(:disabled):hover {
		opacity: 0.9;
	}

	.cp__success {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(34, 181, 115, 0.1);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		font-size: var(--fs-sm);
	}
	.cp__error {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}
	.cp__error-block {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}

	.cp__table-wrap {
		overflow-x: auto;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.05);
	}
	.cp__table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.cp__table thead {
		background-color: rgba(255, 255, 255, 0.02);
	}
	.cp__table th,
	.cp__table td {
		text-align: left;
		padding: 0.7rem 0.9rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		color: var(--color-grey-300);
		vertical-align: middle;
	}
	.cp__table tbody tr:last-child th,
	.cp__table tbody tr:last-child td {
		border-bottom: none;
	}
	.cp__table thead th {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-400);
	}
	.cp__cell-code {
		font-family: ui-monospace, SFMono-Regular, monospace;
		color: var(--color-white);
		letter-spacing: 0.05em;
		font-weight: var(--w-semibold);
	}
	.cp__cell-date {
		color: var(--color-grey-400);
		white-space: nowrap;
	}
	.cp__col-num {
		text-align: right;
		font-variant-numeric: tabular-nums;
		color: var(--color-white);
		font-weight: var(--w-semibold);
	}
	.cp__link {
		color: var(--color-teal);
		text-decoration: none;
		font-family: var(--font-heading);
	}
	.cp__link:hover {
		text-decoration: underline;
	}
	.cp__dash {
		color: var(--color-grey-400);
	}

	.empty {
		text-align: center;
		padding: 2.25rem 1.5rem;
		border: 1px dashed rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-xl);
		background-color: rgba(255, 255, 255, 0.01);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.55rem;
	}
	.empty__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 3.5rem;
		height: 3.5rem;
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

	@media (max-width: 640px) {
		.cp__card {
			padding: 1rem;
		}
	}
</style>
