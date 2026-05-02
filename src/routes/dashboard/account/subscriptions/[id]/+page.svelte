<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import type { components } from '$lib/api/schema';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import PauseCircleIcon from 'phosphor-svelte/lib/PauseCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import ReceiptIcon from 'phosphor-svelte/lib/ReceiptIcon';

	// Canonical types come from the OpenAPI snapshot. The backend's
	// `Subscription`, `MemberSubscriptionInvoice`, and
	// `MemberSubscriptionDetailResponse` schemas now carry every field
	// this page renders (cancel_at / paused_at / pause_resumes_at /
	// trial_end on the subscription, hosted_invoice_url / invoice_pdf on
	// the invoice). No bespoke mirror lives here any more.
	type SubscriptionStatusJson = components['schemas']['SubscriptionStatus'];
	type SubscriptionRow = components['schemas']['Subscription'];
	type SubscriptionDetail = components['schemas']['MemberSubscriptionDetailResponse'];
	// `MemberSubscriptionDetailResponse.plan` ships as the OpenAPI-shaped
	// `PricingPlan` (with optional `description` / `stripe_*` fields). The
	// `/api/pricing/plans` endpoint also flows through the same schema, so
	// `formatInterval` reads from the canonical row shape and avoids the
	// hand-written `$lib/api/types` mirror that requires `description` to
	// be non-optional.
	type SchemaPricingPlan = components['schemas']['PricingPlan'];

	interface SwitchPlanPreview {
		proration_credit_cents: number;
		proration_charge_cents: number;
		immediate_total_cents: number;
		next_invoice_total_cents: number;
		currency: string;
	}

	let detail = $state<SubscriptionDetail | null>(null);
	let plans = $state<SchemaPricingPlan[]>([]);
	let plansLoaded = $state(false);
	let loading = $state(true);
	let error = $state('');
	let actionError = $state('');
	let actionSuccess = $state('');
	let notFound = $state(false);
	let busy = $state(false);

	// Cancel dialog
	let showCancel = $state(false);
	// Pause dialog
	let showPause = $state(false);
	let pauseChoice = $state<'1m' | '3m' | 'indef'>('1m');
	// Switch plan dialog
	let showSwitch = $state(false);
	let switchTargetId = $state<string | null>(null);
	let switchPreview = $state<SwitchPlanPreview | null>(null);
	let switchPreviewLoading = $state(false);
	let switchPreviewError = $state('');

	const subId = $derived(page.params.id ?? '');

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
			case 'PastDue':
				return 'Past Due';
			default:
				return status;
		}
	}

	function formatMoney(
		cents: number | null | undefined,
		currency: string | null | undefined
	): string {
		if (cents == null) return '—';
		const cur = (currency ?? 'usd').toUpperCase();
		try {
			return new Intl.NumberFormat(undefined, { style: 'currency', currency: cur }).format(
				cents / 100
			);
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

	function formatInterval(plan: SchemaPricingPlan | null | undefined): string {
		if (!plan) return '';
		const i = plan.interval;
		if (i === 'one_time') return 'one-time';
		const unit = i === 'year' ? 'year' : i === 'month' ? 'month' : i;
		if (plan.interval_count > 1) return `every ${plan.interval_count} ${unit}s`;
		return `per ${unit}`;
	}

	async function loadDetail() {
		loading = true;
		error = '';
		notFound = false;
		try {
			detail = await api.get<SubscriptionDetail>(
				`/api/member/subscriptions/${encodeURIComponent(subId)}`
			);
		} catch (e) {
			if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
				notFound = true;
			} else {
				error = e instanceof ApiError ? e.message : 'Failed to load subscription';
			}
			detail = null;
		} finally {
			loading = false;
		}
	}

	async function loadPlans() {
		if (plansLoaded) return;
		try {
			const res = await api.get<SchemaPricingPlan[]>('/api/pricing/plans', {
				skipAuth: true
			});
			plans = res.filter((p) => p.is_active);
			plansLoaded = true;
		} catch {
			plans = [];
			plansLoaded = true;
		}
	}

	function clearActionMsg() {
		actionError = '';
		actionSuccess = '';
	}

	async function doCancel() {
		if (!detail) return;
		busy = true;
		clearActionMsg();
		try {
			const updated = await api.post<SubscriptionRow>(
				`/api/member/subscriptions/${encodeURIComponent(subId)}/cancel`
			);
			detail = { ...detail, subscription: updated };
			showCancel = false;
			actionSuccess = 'Subscription set to cancel at the end of the period.';
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Failed to cancel subscription';
		} finally {
			busy = false;
		}
	}

	async function doResumeCancel() {
		if (!detail) return;
		busy = true;
		clearActionMsg();
		try {
			const updated = await api.post<SubscriptionRow>(
				`/api/member/subscriptions/${encodeURIComponent(subId)}/resume`
			);
			detail = { ...detail, subscription: updated };
			actionSuccess = 'Subscription resumed — it will continue renewing.';
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Failed to resume subscription';
		} finally {
			busy = false;
		}
	}

	function computeResumeAt(choice: typeof pauseChoice): string | undefined {
		if (choice === 'indef') return undefined;
		const months = choice === '1m' ? 1 : 3;
		const now = new Date();
		const next = new Date(
			now.getFullYear(),
			now.getMonth() + months,
			now.getDate(),
			now.getHours(),
			now.getMinutes(),
			now.getSeconds()
		);
		return next.toISOString();
	}

	async function doPause() {
		if (!detail) return;
		busy = true;
		clearActionMsg();
		try {
			const resumeAt = computeResumeAt(pauseChoice);
			const body: { resume_at?: string } = {};
			if (resumeAt) body.resume_at = resumeAt;
			const updated = await api.post<SubscriptionRow>(
				`/api/member/subscriptions/${encodeURIComponent(subId)}/pause`,
				body
			);
			detail = { ...detail, subscription: updated };
			showPause = false;
			actionSuccess = 'Subscription paused.';
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Failed to pause subscription';
		} finally {
			busy = false;
		}
	}

	async function doUnpause() {
		if (!detail) return;
		busy = true;
		clearActionMsg();
		try {
			const updated = await api.post<SubscriptionRow>(
				`/api/member/subscriptions/${encodeURIComponent(subId)}/unpause`
			);
			detail = { ...detail, subscription: updated };
			actionSuccess = 'Subscription unpaused.';
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Failed to unpause subscription';
		} finally {
			busy = false;
		}
	}

	async function openSwitch() {
		clearActionMsg();
		switchTargetId = null;
		switchPreview = null;
		switchPreviewError = '';
		await loadPlans();
		showSwitch = true;
	}

	async function previewSwitch(planId: string) {
		switchTargetId = planId;
		switchPreview = null;
		switchPreviewError = '';
		switchPreviewLoading = true;
		try {
			switchPreview = await api.get<SwitchPlanPreview>(
				`/api/member/subscriptions/${encodeURIComponent(subId)}/switch-plan/preview?pricing_plan_id=${encodeURIComponent(planId)}`
			);
		} catch (e) {
			switchPreviewError = e instanceof ApiError ? e.message : 'Failed to preview switch';
		} finally {
			switchPreviewLoading = false;
		}
	}

	async function confirmSwitch() {
		if (!detail || !switchTargetId) return;
		busy = true;
		clearActionMsg();
		try {
			// switch-plan handler reads the `Idempotency-Key` header off the
			// request directly. Provide one so retries are safe.
			const idempotencyKey = (() => {
				if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
					return crypto.randomUUID();
				}
				return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
			})();
			const updated = await api.post<SubscriptionRow>(
				`/api/member/subscriptions/${encodeURIComponent(subId)}/switch-plan`,
				{ pricing_plan_id: switchTargetId, prorate: true },
				{ headers: { 'Idempotency-Key': idempotencyKey } }
			);
			detail = { ...detail, subscription: updated };
			showSwitch = false;
			actionSuccess = 'Plan switched. The new price takes effect immediately.';
			// Re-load to get the new plan name + invoices
			await loadDetail();
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Failed to switch plan';
		} finally {
			busy = false;
		}
	}

	async function openBillingPortal() {
		busy = true;
		clearActionMsg();
		try {
			const res = await api.post<{ url: string }>('/api/member/billing-portal', {});
			window.location.href = res.url;
		} catch (e) {
			actionError = e instanceof ApiError ? e.message : 'Failed to open billing portal';
			busy = false;
		}
	}

	function invoiceTone(status: string): StatusTone {
		const s = status.toLowerCase();
		if (s === 'paid') return 'green';
		if (s === 'open' || s === 'draft') return 'gold';
		if (s === 'uncollectible' || s === 'void') return 'red';
		return 'grey';
	}

	function orderTone(status: string): StatusTone {
		const s = status.toLowerCase();
		if (s === 'completed' || s === 'paid' || s === 'complete') return 'green';
		if (s === 'pending' || s === 'processing') return 'gold';
		if (s === 'refunded' || s === 'partially_refunded') return 'teal';
		if (s === 'cancelled' || s === 'canceled' || s === 'failed') return 'red';
		return 'grey';
	}

	function statusLabelText(status: string): string {
		return status.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
	}

	const sub = $derived(detail?.subscription ?? null);
	const isCancelPending = $derived(sub ? sub.cancel_at != null : false);
	const isPaused = $derived(sub ? sub.status === 'Paused' || sub.paused_at != null : false);
	const isActiveLike = $derived(
		sub ? sub.status === 'Active' || sub.status === 'Trialing' : false
	);
	const isReactivatable = $derived(
		sub
			? sub.status === 'Canceled' || sub.status === 'PastDue' || sub.status === 'Unpaid'
			: false
	);

	onMount(() => {
		void loadDetail();
	});
</script>

<svelte:head><title>Subscription Detail - Precision Options Signals</title></svelte:head>

<section class="sd">
	<a class="sd__back" href={resolve('/dashboard/account/subscriptions')}>
		<ArrowLeftIcon size={14} weight="bold" />
		Back to subscriptions
	</a>

	{#if loading}
		<p class="sd__loading">Loading subscription…</p>
	{:else if notFound}
		<div class="sd__notfound" role="alert">
			<div class="empty__icon"><LightningIcon size={36} weight="duotone" /></div>
			<h2 class="empty__title">Subscription not found</h2>
			<p class="empty__body">
				This subscription may have been deleted, or you may not have permission to view it.
			</p>
			<a class="empty__cta" href={resolve('/dashboard/account/subscriptions')}>
				Back to your subscriptions
			</a>
		</div>
	{:else if error}
		<div class="sd__error" role="alert">{error}</div>
	{:else if detail && sub}
		{@const tone = statusTone(sub.status)}

		<header class="sd__header">
			<div class="sd__header-row">
				<div>
					<h1 class="sd__id">Subscription #{sub.id.slice(0, 8)}</h1>
					<p class="sd__plan-name">
						{detail.plan?.name ??
							(sub.plan === 'Annual' ? 'Annual plan' : 'Monthly plan')}
					</p>
				</div>
				<div class="sd__header-meta">
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
			</div>
		</header>

		{#if actionSuccess}
			<div class="sd__success" role="status">{actionSuccess}</div>
		{/if}
		{#if actionError}
			<div class="sd__error" role="alert">{actionError}</div>
		{/if}

		<div class="sd__grid">
			<!-- A) Overview -->
			<section class="sd__card sd__card--overview" aria-labelledby="overview-title">
				<h2 id="overview-title" class="sd__section-title">Overview</h2>

				<dl class="sd__kv">
					<div>
						<dt>Plan</dt>
						<dd>
							{detail.plan?.name ?? (sub.plan === 'Annual' ? 'Annual' : 'Monthly')}
						</dd>
					</div>
					<div>
						<dt>Amount</dt>
						<dd class="sd__num">
							{#if detail.plan}
								{formatMoney(detail.plan.amount_cents, detail.plan.currency)}
								<span class="sd__interval">{formatInterval(detail.plan)}</span>
							{:else if sub.grandfathered_price_cents != null}
								{formatMoney(
									sub.grandfathered_price_cents,
									sub.grandfathered_currency
								)}
							{:else}
								—
							{/if}
						</dd>
					</div>
					<div>
						<dt>Status</dt>
						<dd>
							<span class="badge badge--{tone}">
								{#if tone === 'green'}
									<CheckCircleIcon size={12} weight="fill" />
								{:else if tone === 'teal'}
									<ClockIcon size={12} weight="fill" />
								{:else if tone === 'gold'}
									<PauseCircleIcon size={12} weight="fill" />
								{:else if tone === 'red'}
									<WarningIcon size={12} weight="fill" />
								{:else}
									<XCircleIcon size={12} weight="fill" />
								{/if}
								{statusLabel(sub.status)}
							</span>
						</dd>
					</div>
					<div>
						<dt>Current period</dt>
						<dd>
							{formatDate(sub.current_period_start)} → {formatDate(
								sub.current_period_end
							)}
						</dd>
					</div>
					{#if sub.trial_end}
						<div>
							<dt>Trial ends</dt>
							<dd>{formatDate(sub.trial_end)}</dd>
						</div>
					{/if}
				</dl>

				{#if isCancelPending}
					<div class="sd__notice sd__notice--gold">
						<WarningIcon size={16} weight="fill" />
						<span
							>Will cancel on {formatDate(
								sub.cancel_at ?? sub.current_period_end
							)}</span
						>
					</div>
				{/if}

				{#if isPaused}
					<div class="sd__notice sd__notice--gold">
						<PauseCircleIcon size={16} weight="fill" />
						<span>
							{#if sub.pause_resumes_at}
								Paused — resumes on {formatDate(sub.pause_resumes_at)}
							{:else}
								Paused indefinitely
							{/if}
						</span>
					</div>
				{/if}
			</section>

			<!-- B) Actions -->
			<section class="sd__card sd__card--actions" aria-labelledby="actions-title">
				<h2 id="actions-title" class="sd__section-title">Actions</h2>

				<div class="sd__actions">
					{#if isActiveLike && !isCancelPending}
						<button
							type="button"
							class="btn btn--danger"
							onclick={() => (showCancel = true)}
							disabled={busy}
						>
							<XCircleIcon size={16} weight="bold" />
							Cancel Subscription
						</button>
						<button
							type="button"
							class="btn btn--ghost"
							onclick={() => (showPause = true)}
							disabled={busy}
						>
							<PauseCircleIcon size={16} weight="bold" />
							Pause Subscription
						</button>
						<button
							type="button"
							class="btn btn--primary"
							onclick={openSwitch}
							disabled={busy}
						>
							<ArrowRightIcon size={16} weight="bold" />
							Switch Plan
						</button>
					{:else if isActiveLike && isCancelPending}
						<button
							type="button"
							class="btn btn--primary"
							onclick={doResumeCancel}
							disabled={busy}
						>
							<CheckCircleIcon size={16} weight="bold" />
							{busy ? 'Resuming…' : 'Resume Subscription'}
						</button>
					{:else if isPaused}
						<button
							type="button"
							class="btn btn--primary"
							onclick={doUnpause}
							disabled={busy}
						>
							<CheckCircleIcon size={16} weight="bold" />
							{busy ? 'Unpausing…' : 'Unpause Subscription'}
						</button>
					{:else if isReactivatable}
						<button
							type="button"
							class="btn btn--primary"
							onclick={openBillingPortal}
							disabled={busy}
						>
							<ArrowRightIcon size={16} weight="bold" />
							{busy ? 'Opening…' : 'Reactivate via Billing Portal'}
						</button>
					{:else}
						<p class="sd__muted">
							No actions available for this subscription right now.
						</p>
					{/if}
				</div>
			</section>
		</div>

		<!-- C) Invoices -->
		<section class="sd__card" aria-labelledby="invoices-title">
			<h2 id="invoices-title" class="sd__section-title">Invoices</h2>
			{#if detail.invoices.length === 0}
				<p class="sd__muted">No invoices yet.</p>
			{:else}
				<div class="sd__table-wrap">
					<table class="sd__table">
						<thead>
							<tr>
								<th scope="col">Date</th>
								<th scope="col">Description</th>
								<th scope="col" class="sd__col-num">Amount</th>
								<th scope="col">Status</th>
								<th scope="col" class="sd__col-actions">Actions</th>
							</tr>
						</thead>
						<tbody>
							{#each detail.invoices as inv (inv.id)}
								{@const itone = invoiceTone(inv.status)}
								<tr>
									<th scope="row">{formatDate(inv.paid_at ?? inv.created_at)}</th>
									<td class="sd__inv-desc">
										{#if inv.period_start && inv.period_end}
											Billing period {formatDate(inv.period_start)} – {formatDate(
												inv.period_end
											)}
										{:else}
											Subscription invoice
										{/if}
										<span class="sd__inv-id">{inv.stripe_invoice_id}</span>
									</td>
									<td class="sd__col-num"
										>{formatMoney(inv.amount_due_cents, inv.currency)}</td
									>
									<td>
										<span class="badge badge--{itone}"
											>{statusLabelText(inv.status)}</span
										>
									</td>
									<td class="sd__col-actions">
										{#if inv.hosted_invoice_url}
											<!-- eslint-disable svelte/no-navigation-without-resolve -->
											<!-- hosted_invoice_url is a Stripe-hosted absolute URL, not a SvelteKit typed route. -->
											<a
												class="btn-mini"
												href={inv.hosted_invoice_url}
												target="_blank"
												rel="noopener noreferrer"
												aria-label="View receipt for invoice {inv.stripe_invoice_id}"
											>
												<ReceiptIcon size={14} weight="bold" />
												View Receipt
											</a>
											<!-- eslint-enable svelte/no-navigation-without-resolve -->
										{:else}
											<span class="sd__dash">—</span>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>

		<!-- D) Related Orders -->
		{#if detail.related_orders.length > 0}
			<section class="sd__card" aria-labelledby="orders-title">
				<h2 id="orders-title" class="sd__section-title">Related orders</h2>
				<div class="sd__table-wrap">
					<table class="sd__table">
						<thead>
							<tr>
								<th scope="col">Order #</th>
								<th scope="col">Date</th>
								<th scope="col" class="sd__col-num">Total</th>
								<th scope="col">Status</th>
								<th scope="col" class="sd__col-actions">View</th>
							</tr>
						</thead>
						<tbody>
							{#each detail.related_orders as o (o.id)}
								{@const otone = orderTone(o.status)}
								<tr>
									<th scope="row">
										<a
											class="sd__inline-link"
											href={resolve('/dashboard/account/orders/[id]', {
												id: o.id
											})}
										>
											#{o.number}
										</a>
									</th>
									<td>{formatDate(o.placed_at ?? o.created_at)}</td>
									<td class="sd__col-num"
										>{formatMoney(o.total_cents, o.currency)}</td
									>
									<td>
										<span class="badge badge--{otone}"
											>{statusLabelText(o.status)}</span
										>
									</td>
									<td class="sd__col-actions">
										<a
											class="btn-mini"
											href={resolve('/dashboard/account/orders/[id]', {
												id: o.id
											})}
										>
											<EyeIcon size={14} weight="bold" />
											View
										</a>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</section>
		{/if}
	{/if}
</section>

<!-- Cancel dialog -->
{#if showCancel && sub}
	<div
		class="dialog__backdrop"
		role="dialog"
		aria-modal="true"
		aria-labelledby="cancel-title"
		onclick={(e) => {
			if (e.target === e.currentTarget && !busy) showCancel = false;
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape' && !busy) showCancel = false;
		}}
		tabindex="-1"
	>
		<div class="dialog">
			<h3 id="cancel-title" class="dialog__title">Cancel subscription?</h3>
			<p class="dialog__body">
				Are you sure you want to cancel? You'll keep access until {formatDate(
					sub.current_period_end
				)}.
			</p>
			<div class="dialog__actions">
				<button
					type="button"
					class="btn btn--ghost"
					onclick={() => (showCancel = false)}
					disabled={busy}
				>
					Keep subscription
				</button>
				<button type="button" class="btn btn--danger" onclick={doCancel} disabled={busy}>
					{busy ? 'Cancelling…' : 'Yes, cancel'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Pause dialog -->
{#if showPause && sub}
	<div
		class="dialog__backdrop"
		role="dialog"
		aria-modal="true"
		aria-labelledby="pause-title"
		onclick={(e) => {
			if (e.target === e.currentTarget && !busy) showPause = false;
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape' && !busy) showPause = false;
		}}
		tabindex="-1"
	>
		<div class="dialog">
			<h3 id="pause-title" class="dialog__title">Pause subscription</h3>
			<p class="dialog__body">When should your subscription resume?</p>
			<fieldset class="dialog__radios">
				<legend class="visually-hidden">Resume window</legend>
				<label class="radio">
					<input type="radio" name="pause-window" value="1m" bind:group={pauseChoice} />
					<span>Resume in 1 month</span>
				</label>
				<label class="radio">
					<input type="radio" name="pause-window" value="3m" bind:group={pauseChoice} />
					<span>Resume in 3 months</span>
				</label>
				<label class="radio">
					<input
						type="radio"
						name="pause-window"
						value="indef"
						bind:group={pauseChoice}
					/>
					<span>Indefinite (manual resume)</span>
				</label>
			</fieldset>
			<div class="dialog__actions">
				<button
					type="button"
					class="btn btn--ghost"
					onclick={() => (showPause = false)}
					disabled={busy}
				>
					Cancel
				</button>
				<button type="button" class="btn btn--primary" onclick={doPause} disabled={busy}>
					{busy ? 'Pausing…' : 'Pause subscription'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Switch plan dialog -->
{#if showSwitch && sub}
	<div
		class="dialog__backdrop"
		role="dialog"
		aria-modal="true"
		aria-labelledby="switch-title"
		onclick={(e) => {
			if (e.target === e.currentTarget && !busy) showSwitch = false;
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape' && !busy) showSwitch = false;
		}}
		tabindex="-1"
	>
		<div class="dialog dialog--wide">
			<h3 id="switch-title" class="dialog__title">Switch plan</h3>
			<p class="dialog__body">
				Pick a target plan and preview the proration before confirming. Your switch is
				immediate and prorated against your current period.
			</p>

			{#if !plansLoaded}
				<p class="sd__muted">Loading plans…</p>
			{:else if plans.length === 0}
				<p class="sd__muted">No alternative plans available right now.</p>
			{:else}
				<ul class="plans" role="list">
					{#each plans as p (p.id)}
						{@const isCurrent = p.id === sub.pricing_plan_id}
						{@const isSelected = p.id === switchTargetId}
						<li
							class="plan"
							class:plan--selected={isSelected}
							class:plan--current={isCurrent}
						>
							<div class="plan__main">
								<div class="plan__name-row">
									<span class="plan__name">{p.name}</span>
									{#if isCurrent}<span class="badge badge--teal">Current</span
										>{/if}
								</div>
								<span class="plan__price">
									{formatMoney(p.amount_cents, p.currency)}
									<span class="plan__interval">{formatInterval(p)}</span>
								</span>
								{#if p.description}
									<p class="plan__desc">{p.description}</p>
								{/if}
							</div>
							<button
								type="button"
								class="btn btn--ghost btn--small"
								onclick={() => previewSwitch(p.id)}
								disabled={isCurrent || busy || switchPreviewLoading}
							>
								{#if isSelected && switchPreviewLoading}Loading…{:else}Preview{/if}
							</button>
						</li>
					{/each}
				</ul>

				{#if switchTargetId}
					<div class="preview">
						{#if switchPreviewLoading}
							<p class="sd__muted">Calculating proration…</p>
						{:else if switchPreviewError}
							<p class="dialog__error">{switchPreviewError}</p>
						{:else if switchPreview}
							<p class="preview__line">
								You'll be credited
								<strong>
									{formatMoney(
										switchPreview.proration_credit_cents,
										switchPreview.currency
									)}
								</strong>
								today and charged
								<strong>
									{formatMoney(
										switchPreview.next_invoice_total_cents,
										switchPreview.currency
									)}
								</strong>
								on
								<strong>{formatDate(sub.current_period_end)}</strong>.
							</p>
							<dl class="preview__details">
								<div>
									<dt>Immediate amount</dt>
									<dd>
										{formatMoney(
											switchPreview.immediate_total_cents,
											switchPreview.currency
										)}
									</dd>
								</div>
								<div>
									<dt>Proration credit</dt>
									<dd>
										{formatMoney(
											switchPreview.proration_credit_cents,
											switchPreview.currency
										)}
									</dd>
								</div>
								<div>
									<dt>Proration charge</dt>
									<dd>
										{formatMoney(
											switchPreview.proration_charge_cents,
											switchPreview.currency
										)}
									</dd>
								</div>
							</dl>
						{/if}
					</div>
				{/if}
			{/if}

			<div class="dialog__actions">
				<button
					type="button"
					class="btn btn--ghost"
					onclick={() => (showSwitch = false)}
					disabled={busy}
				>
					Cancel
				</button>
				<button
					type="button"
					class="btn btn--primary"
					onclick={confirmSwitch}
					disabled={busy || !switchTargetId || !switchPreview}
				>
					{busy ? 'Switching…' : 'Confirm switch'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.sd {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.sd__back {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
		align-self: flex-start;
	}
	.sd__back:hover {
		text-decoration: underline;
	}
	.sd__loading {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		padding: 1.5rem 0;
	}
	.sd__error {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}
	.sd__success {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(34, 181, 115, 0.1);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		font-size: var(--fs-sm);
	}
	.sd__notfound {
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

	.sd__header {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-top: 2px solid var(--color-teal);
		border-radius: var(--radius-xl);
		padding: 1.25rem 1.5rem;
	}
	.sd__header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
	}
	.sd__id {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.sd__plan-name {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		margin-top: 0.25rem;
	}
	.sd__header-meta {
		display: inline-flex;
		align-items: center;
		gap: 1rem;
	}

	.sd__grid {
		display: grid;
		grid-template-columns: 2fr 1fr;
		gap: 1.25rem;
	}
	.sd__card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.sd__section-title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.sd__kv {
		margin: 0;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.85rem 1.25rem;
	}
	.sd__kv > div {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	.sd__kv dt {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.sd__kv dd {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--color-white);
	}
	.sd__num {
		font-variant-numeric: tabular-nums;
	}
	.sd__interval {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		margin-left: 0.35rem;
	}

	.sd__notice {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
	}
	.sd__notice--gold {
		background-color: rgba(212, 168, 67, 0.12);
		border: 1px solid rgba(212, 168, 67, 0.3);
		color: var(--color-gold);
	}

	.sd__actions {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.sd__muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.6rem 0.9rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		border: 1px solid transparent;
		transition:
			opacity 200ms var(--ease-out),
			background-color 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}
	.btn--small {
		padding: 0.4rem 0.7rem;
		font-size: var(--fs-xs);
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
	}
	.btn--primary:not(:disabled):hover {
		opacity: 0.9;
	}
	.btn--ghost {
		background-color: transparent;
		border-color: rgba(255, 255, 255, 0.12);
		color: var(--color-grey-300);
	}
	.btn--ghost:not(:disabled):hover {
		color: var(--color-white);
		border-color: rgba(15, 164, 175, 0.3);
		background-color: rgba(15, 164, 175, 0.06);
	}
	.btn--danger {
		background-color: rgba(224, 72, 72, 0.1);
		border-color: rgba(224, 72, 72, 0.3);
		color: var(--color-red);
	}
	.btn--danger:not(:disabled):hover {
		background-color: rgba(224, 72, 72, 0.18);
	}

	.btn-mini {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.35rem 0.65rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.25);
		text-decoration: none;
	}
	.btn-mini:hover {
		background-color: rgba(15, 164, 175, 0.2);
	}

	.sd__table-wrap {
		overflow-x: auto;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.05);
	}
	.sd__table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.sd__table thead {
		background-color: rgba(255, 255, 255, 0.02);
	}
	.sd__table th,
	.sd__table td {
		text-align: left;
		padding: 0.7rem 0.9rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		color: var(--color-grey-300);
		vertical-align: middle;
	}
	.sd__table tbody tr:last-child th,
	.sd__table tbody tr:last-child td {
		border-bottom: none;
	}
	.sd__table thead th {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-400);
	}
	.sd__col-num {
		text-align: right;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
		color: var(--color-white);
		font-weight: var(--w-semibold);
	}
	.sd__col-actions {
		text-align: right;
		width: 1%;
		white-space: nowrap;
	}
	.sd__inv-desc {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		color: var(--color-white);
	}
	.sd__inv-id {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		font-family: ui-monospace, SFMono-Regular, monospace;
	}
	.sd__dash {
		color: var(--color-grey-400);
	}
	.sd__inline-link {
		color: var(--color-white);
		text-decoration: none;
		font-family: var(--font-heading);
		font-weight: var(--w-semibold);
	}
	.sd__inline-link:hover {
		color: var(--color-teal);
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

	/* Dialog */
	.dialog__backdrop {
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.55);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		z-index: 200;
	}
	.dialog {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		width: 100%;
		max-width: 28rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
	}
	.dialog--wide {
		max-width: 38rem;
	}
	.dialog__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.dialog__body {
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
	}
	.dialog__error {
		color: var(--color-red);
		font-size: var(--fs-sm);
	}
	.dialog__actions {
		display: flex;
		gap: 0.65rem;
		justify-content: flex-end;
		flex-wrap: wrap;
	}
	.dialog__radios {
		border: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.radio {
		display: inline-flex;
		align-items: center;
		gap: 0.55rem;
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		cursor: pointer;
		padding: 0.4rem 0.5rem;
		border-radius: var(--radius-lg);
	}
	.radio:hover {
		background-color: rgba(15, 164, 175, 0.06);
	}
	.radio input {
		accent-color: var(--color-teal);
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		border: 0;
	}

	.plans {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 18rem;
		overflow-y: auto;
	}
	.plan {
		display: flex;
		gap: 1rem;
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.07);
		background-color: rgba(255, 255, 255, 0.02);
		align-items: center;
		justify-content: space-between;
	}
	.plan--selected {
		border-color: rgba(15, 164, 175, 0.45);
		background-color: rgba(15, 164, 175, 0.06);
	}
	.plan--current {
		opacity: 0.65;
	}
	.plan__main {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		min-width: 0;
	}
	.plan__name-row {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
	}
	.plan__name {
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
	}
	.plan__price {
		color: var(--color-white);
		font-variant-numeric: tabular-nums;
		font-size: var(--fs-sm);
	}
	.plan__interval {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		margin-left: 0.25rem;
	}
	.plan__desc {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
	}

	.preview {
		padding: 0.85rem 1rem;
		background-color: rgba(15, 164, 175, 0.06);
		border: 1px solid rgba(15, 164, 175, 0.18);
		border-radius: var(--radius-lg);
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.preview__line {
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
	}
	.preview__line strong {
		color: var(--color-white);
	}
	.preview__details {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.5rem;
		margin: 0;
	}
	.preview__details > div {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.preview__details dt {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.preview__details dd {
		margin: 0;
		color: var(--color-white);
		font-variant-numeric: tabular-nums;
		font-size: var(--fs-sm);
	}

	@media (max-width: 900px) {
		.sd__grid {
			grid-template-columns: 1fr;
		}
	}
	@media (max-width: 640px) {
		.sd__header,
		.sd__card {
			padding: 1rem;
		}
		.sd__kv {
			grid-template-columns: 1fr;
		}
		.preview__details {
			grid-template-columns: 1fr;
		}
	}
</style>
