<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import type { PaginatedResponse } from '$lib/api/types';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import PauseCircleIcon from 'phosphor-svelte/lib/PauseCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';

	type SubscriptionStatusJson =
		| 'Active'
		| 'Canceled'
		| 'PastDue'
		| 'Paused'
		| 'Trialing'
		| 'Unpaid';

	type SubscriptionPlanJson = 'Monthly' | 'Annual';

	interface MemberSubscriptionListItem {
		id: string;
		plan: SubscriptionPlanJson;
		status: SubscriptionStatusJson;
		current_period_start: string;
		current_period_end: string;
		paused_at?: string | null;
		pause_resumes_at?: string | null;
		cancel_at_period_end: boolean;
		pricing_plan_id?: string | null;
		plan_name?: string | null;
		amount_cents?: number | null;
		currency?: string | null;
		created_at: string;
	}

	const PER_PAGE = 20;

	let envelope = $state<PaginatedResponse<MemberSubscriptionListItem> | null>(null);
	let loading = $state(true);
	let error = $state('');
	let page = $state(1);

	type StatusTone = 'green' | 'teal' | 'gold' | 'red' | 'grey';

	function statusTone(status: SubscriptionStatusJson): StatusTone {
		switch (status) {
			case 'Active':
				return 'green';
			case 'Trialing':
				return 'teal';
			case 'Paused':
				return 'gold';
			case 'PastDue':
			case 'Unpaid':
				return 'red';
			case 'Canceled':
				return 'grey';
			default:
				return 'grey';
		}
	}

	function statusLabel(status: SubscriptionStatusJson): string {
		switch (status) {
			case 'Active':
				return 'Active';
			case 'Canceled':
				return 'Canceled';
			case 'PastDue':
				return 'Past Due';
			case 'Paused':
				return 'Paused';
			case 'Trialing':
				return 'Trialing';
			case 'Unpaid':
				return 'Unpaid';
			default:
				return status;
		}
	}

	function planLabel(s: MemberSubscriptionListItem): string {
		if (s.plan_name) return s.plan_name;
		return s.plan === 'Annual' ? 'Annual' : 'Monthly';
	}

	function formatMoney(cents: number | null | undefined, currency: string | null | undefined): string {
		if (cents == null) return '—';
		const cur = (currency ?? 'usd').toUpperCase();
		try {
			return new Intl.NumberFormat(undefined, {
				style: 'currency',
				currency: cur
			}).format(cents / 100);
		} catch {
			return `${cur} ${(cents / 100).toFixed(2)}`;
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

	function nextPaymentLabel(s: MemberSubscriptionListItem): string {
		if (s.status === 'Active' || s.status === 'Trialing') {
			return formatDate(s.current_period_end);
		}
		return '—';
	}

	function shortId(id: string): string {
		return `#${id.slice(0, 8)}`;
	}

	async function load(p: number) {
		loading = true;
		error = '';
		try {
			envelope = await api.get<PaginatedResponse<MemberSubscriptionListItem>>(
				`/api/member/subscriptions?page=${p}&per_page=${PER_PAGE}`
			);
			page = envelope.page;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to load subscriptions';
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

	const subs = $derived(envelope?.data ?? []);
	const totalPages = $derived(envelope?.total_pages ?? 1);
</script>

<svelte:head><title>My Subscriptions - Precision Options Signals</title></svelte:head>

<section class="subs">
	<header class="subs__header">
		<h1 class="subs__title">My Subscriptions</h1>
		<p class="subs__sub">Manage your active and past memberships.</p>
	</header>

	{#if loading && !envelope}
		<p class="subs__loading">Loading your subscriptions…</p>
	{:else if error}
		<div class="subs__error" role="alert">{error}</div>
	{:else if subs.length === 0}
		<div class="empty">
			<div class="empty__icon"><LightningIcon size={36} weight="duotone" /></div>
			<h3 class="empty__title">No subscriptions yet</h3>
			<p class="empty__body">
				Pick a plan and unlock real-time signals, courses, and the full member experience.
			</p>
			<a href={resolve('/pricing')} class="empty__cta">
				View Plans <ArrowRightIcon size={14} weight="bold" />
			</a>
		</div>
	{:else}
		<div class="subs__table-wrap" role="region" aria-label="Subscriptions">
			<table class="subs__table">
				<thead>
					<tr>
						<th scope="col">Subscription</th>
						<th scope="col">Status</th>
						<th scope="col">Product</th>
						<th scope="col">Next Payment</th>
						<th scope="col">Total</th>
						<th scope="col" class="subs__col-actions">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each subs as sub (sub.id)}
						{@const tone = statusTone(sub.status)}
						<tr>
							<th scope="row" class="subs__cell-id">
								<a
									class="subs__id-link"
									href={resolve('/dashboard/account/subscriptions/[id]', { id: sub.id })}
								>
									{shortId(sub.id)}
								</a>
							</th>
							<td>
								<span
									class="badge badge--{tone}"
									aria-label="Status: {statusLabel(sub.status)}"
								>
									<span class="badge__icon" aria-hidden="true">
										{#if tone === 'green'}
											<CheckCircleIcon size={14} weight="fill" />
										{:else if tone === 'teal'}
											<ClockIcon size={14} weight="fill" />
										{:else if tone === 'gold'}
											<PauseCircleIcon size={14} weight="fill" />
										{:else if tone === 'red'}
											<WarningIcon size={14} weight="fill" />
										{:else}
											<XCircleIcon size={14} weight="fill" />
										{/if}
									</span>
									<span>{statusLabel(sub.status)}</span>
								</span>
								{#if sub.cancel_at_period_end}
									<span class="subs__hint">Cancels {formatDate(sub.current_period_end)}</span>
								{/if}
							</td>
							<td class="subs__cell-product">{planLabel(sub)}</td>
							<td class="subs__cell-date">{nextPaymentLabel(sub)}</td>
							<td class="subs__cell-num">{formatMoney(sub.amount_cents, sub.currency)}</td>
							<td class="subs__col-actions">
								<a
									class="subs__view"
									href={resolve('/dashboard/account/subscriptions/[id]', { id: sub.id })}
									aria-label="View subscription {shortId(sub.id)}"
								>
									<EyeIcon size={14} weight="bold" />
									View
								</a>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>

			<ul class="subs__cards" role="list">
				{#each subs as sub (sub.id)}
					{@const tone = statusTone(sub.status)}
					<li class="card">
						<div class="card__top">
							<a
								class="card__id"
								href={resolve('/dashboard/account/subscriptions/[id]', { id: sub.id })}
							>
								{shortId(sub.id)}
							</a>
							<span class="badge badge--{tone}">
								<span class="badge__icon" aria-hidden="true">
									{#if tone === 'green'}
										<CheckCircleIcon size={14} weight="fill" />
									{:else if tone === 'teal'}
										<ClockIcon size={14} weight="fill" />
									{:else if tone === 'gold'}
										<PauseCircleIcon size={14} weight="fill" />
									{:else if tone === 'red'}
										<WarningIcon size={14} weight="fill" />
									{:else}
										<XCircleIcon size={14} weight="fill" />
									{/if}
								</span>
								<span>{statusLabel(sub.status)}</span>
							</span>
						</div>
						<dl class="card__rows">
							<div>
								<dt>Product</dt>
								<dd>{planLabel(sub)}</dd>
							</div>
							<div>
								<dt>Next payment</dt>
								<dd>{nextPaymentLabel(sub)}</dd>
							</div>
							<div>
								<dt>Total</dt>
								<dd class="card__num-val">{formatMoney(sub.amount_cents, sub.currency)}</dd>
							</div>
						</dl>
						{#if sub.cancel_at_period_end}
							<p class="card__hint">Cancels on {formatDate(sub.current_period_end)}</p>
						{/if}
						<a
							class="card__view"
							href={resolve('/dashboard/account/subscriptions/[id]', { id: sub.id })}
						>
							<EyeIcon size={14} weight="bold" />
							View subscription
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
	.subs { display: flex; flex-direction: column; gap: 1.25rem; }
	.subs__header { display: flex; flex-direction: column; gap: 0.25rem; }
	.subs__title { font-size: var(--fs-2xl); font-weight: var(--w-bold); color: var(--color-white); font-family: var(--font-heading); }
	.subs__sub { color: var(--color-grey-400); font-size: var(--fs-sm); }
	.subs__loading { color: var(--color-grey-400); font-size: var(--fs-sm); padding: 1.5rem 0; }
	.subs__error { padding: 0.85rem 1rem; border-radius: var(--radius-lg); background-color: rgba(224, 72, 72, 0.1); border: 1px solid rgba(224, 72, 72, 0.25); color: var(--color-red); font-size: var(--fs-sm); }

	.subs__table-wrap { background-color: var(--color-navy-mid); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-xl); overflow: hidden; }
	.subs__table { width: 100%; border-collapse: collapse; font-size: var(--fs-sm); }
	.subs__table thead { background-color: rgba(255, 255, 255, 0.02); }
	.subs__table th, .subs__table td { text-align: left; padding: 0.85rem 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.05); vertical-align: middle; color: var(--color-grey-300); }
	.subs__table tbody tr:last-child th, .subs__table tbody tr:last-child td { border-bottom: none; }
	.subs__table thead th { font-size: var(--fs-xs); font-weight: var(--w-semibold); text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-grey-400); }
	.subs__cell-id { font-weight: var(--w-semibold); }
	.subs__id-link { color: var(--color-white); text-decoration: none; font-family: var(--font-heading); }
	.subs__id-link:hover { color: var(--color-teal); }
	.subs__cell-product { color: var(--color-white); font-weight: var(--w-medium); }
	.subs__cell-date { color: var(--color-grey-400); white-space: nowrap; }
	.subs__cell-num { font-variant-numeric: tabular-nums; color: var(--color-white); font-weight: var(--w-semibold); }
	.subs__col-actions { text-align: right; width: 1%; white-space: nowrap; }
	.subs__view { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.4rem 0.75rem; border-radius: var(--radius-lg); font-size: var(--fs-xs); font-weight: var(--w-semibold); color: var(--color-teal); background-color: rgba(15, 164, 175, 0.1); border: 1px solid rgba(15, 164, 175, 0.25); text-decoration: none; transition: background-color 200ms var(--ease-out); }
	.subs__view:hover { background-color: rgba(15, 164, 175, 0.2); }
	.subs__hint { display: block; margin-top: 0.25rem; font-size: var(--fs-xs); color: var(--color-grey-400); }

	.badge { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.25rem 0.65rem; border-radius: var(--radius-full); font-size: var(--fs-xs); font-weight: var(--w-semibold); letter-spacing: 0.02em; white-space: nowrap; }
	.badge__icon { display: inline-flex; align-items: center; }
	.badge--green { background-color: rgba(34, 181, 115, 0.15); color: var(--color-green); }
	.badge--gold { background-color: rgba(212, 168, 67, 0.18); color: var(--color-gold); }
	.badge--teal { background-color: rgba(15, 164, 175, 0.15); color: var(--color-teal); }
	.badge--red { background-color: rgba(224, 72, 72, 0.15); color: var(--color-red); }
	.badge--grey { background-color: rgba(255, 255, 255, 0.06); color: var(--color-grey-300); }

	.subs__cards { display: none; list-style: none; padding: 0; margin: 0; flex-direction: column; gap: 0.6rem; }
	.card { padding: 0.9rem 1rem; background-color: rgba(255, 255, 255, 0.02); border: 1px solid rgba(255, 255, 255, 0.05); border-radius: var(--radius-lg); display: flex; flex-direction: column; gap: 0.65rem; }
	.card__top { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
	.card__id { font-family: var(--font-heading); color: var(--color-white); text-decoration: none; font-weight: var(--w-semibold); }
	.card__rows { display: grid; grid-template-columns: 1fr 1fr; gap: 0.5rem; margin: 0; }
	.card__rows dt { font-size: var(--fs-xs); color: var(--color-grey-400); text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 0.15rem; }
	.card__rows dd { margin: 0; font-size: var(--fs-sm); color: var(--color-grey-300); }
	.card__num-val { font-variant-numeric: tabular-nums; color: var(--color-white); font-weight: var(--w-semibold); }
	.card__hint { font-size: var(--fs-xs); color: var(--color-grey-400); }
	.card__view { display: inline-flex; align-items: center; justify-content: center; gap: 0.4rem; padding: 0.55rem 0.75rem; border-radius: var(--radius-lg); background-color: rgba(15, 164, 175, 0.1); color: var(--color-teal); border: 1px solid rgba(15, 164, 175, 0.25); text-decoration: none; font-size: var(--fs-sm); font-weight: var(--w-semibold); }

	.empty { text-align: center; padding: 3rem 1.5rem; border: 1px dashed rgba(255, 255, 255, 0.12); border-radius: var(--radius-xl); background-color: rgba(255, 255, 255, 0.01); display: flex; flex-direction: column; align-items: center; gap: 0.65rem; }
	.empty__icon { display: inline-flex; align-items: center; justify-content: center; width: 4rem; height: 4rem; border-radius: var(--radius-full); background-color: rgba(15, 164, 175, 0.1); color: var(--color-teal); }
	.empty__title { font-size: var(--fs-md); font-weight: var(--w-semibold); color: var(--color-white); font-family: var(--font-heading); }
	.empty__body { font-size: var(--fs-sm); color: var(--color-grey-400); max-width: 22rem; }
	.empty__cta { display: inline-flex; align-items: center; gap: 0.4rem; margin-top: 0.5rem; padding: 0.55rem 1.1rem; font-size: var(--fs-sm); font-weight: var(--w-semibold); color: var(--color-teal); background-color: rgba(15, 164, 175, 0.1); border: 1px solid rgba(15, 164, 175, 0.25); border-radius: var(--radius-lg); text-decoration: none; }

	.pager { display: flex; align-items: center; justify-content: center; gap: 1rem; padding: 0.5rem 0; }
	.pager__btn { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.5rem 0.85rem; border-radius: var(--radius-lg); background-color: var(--color-navy-mid); border: 1px solid rgba(255, 255, 255, 0.06); color: var(--color-grey-300); font-size: var(--fs-sm); font-weight: var(--w-medium); cursor: pointer; transition: background-color 200ms var(--ease-out), color 200ms var(--ease-out); }
	.pager__btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.pager__btn:not(:disabled):hover { color: var(--color-white); background-color: rgba(15, 164, 175, 0.1); border-color: rgba(15, 164, 175, 0.25); }
	.pager__status { font-size: var(--fs-sm); color: var(--color-grey-400); font-variant-numeric: tabular-nums; }

	@media (max-width: 640px) {
		.subs__table { display: none; }
		.subs__cards { display: flex; padding: 0.6rem; }
	}
</style>
