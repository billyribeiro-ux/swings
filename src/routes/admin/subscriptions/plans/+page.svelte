<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import type {
		AdminUpdatePricingPlanResponse,
		CreatePricingPlanPayload,
		PricingPlan,
		PricingPlanPriceLogEntry,
		PricingStripeRolloutAudience,
		UpdatePricingPlanPayload
	} from '$lib/api/types';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import StarIcon from 'phosphor-svelte/lib/StarIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';

	type BillingInterval = 'month' | 'year';

	let plans = $state<PricingPlan[]>([]);
	let priceLog = $state<PricingPlanPriceLogEntry[]>([]);
	let loading = $state(true);
	let saving = $state<string | null>(null);
	let editingId = $state<string | null>(null);
	let showNewForm = $state(false);
	let logOpen = $state(false);
	let lastStripeRolloutSummary = $state<string | null>(null);

	// Preview counts returned by /rollout-preview before the user commits.
	type RolloutPreview = {
		total_in_audience: number;
		would_update: number;
		would_skip_grandfathered: number;
		current_amount_cents: number;
		currency: string;
	};
	let rolloutPreview = $state<RolloutPreview | null>(null);
	let rolloutPreviewLoading = $state(false);
	let showRolloutConfirm = $state(false);

	let editDraft = $state<{
		name: string;
		amount_dollars: string;
		interval: BillingInterval;
		features: string;
		trial_days: string;
		stripe_price_id: string;
		push_to_stripe_subscribers: boolean;
		rollout_audience: PricingStripeRolloutAudience;
	}>({
		name: '',
		amount_dollars: '',
		interval: 'month',
		features: '',
		trial_days: '0',
		stripe_price_id: '',
		push_to_stripe_subscribers: false,
		rollout_audience: 'linked_subscriptions_only'
	});

	let newPlan = $state<{
		name: string;
		amount_dollars: string;
		interval: BillingInterval;
		features: string;
		trial_days: string;
		stripe_price_id: string;
	}>({
		name: '',
		amount_dollars: '',
		interval: 'month',
		features: '',
		trial_days: '0',
		stripe_price_id: ''
	});

	async function loadPlans() {
		loading = true;
		try {
			plans = await api.get<PricingPlan[]>('/api/admin/pricing/plans');
		} catch {
			plans = [];
		} finally {
			loading = false;
		}
	}

	async function loadPriceLog() {
		try {
			priceLog = await api.get<PricingPlanPriceLogEntry[]>(
				'/api/admin/pricing/plans/price-log'
			);
		} catch {
			priceLog = [];
		}
	}

	onMount(() => {
		loadPlans();
		loadPriceLog();
	});

	function startEdit(plan: PricingPlan) {
		editingId = plan.id;
		lastStripeRolloutSummary = null;
		rolloutPreview = null;
		showRolloutConfirm = false;
		editDraft = {
			name: plan.name,
			amount_dollars: (plan.amount_cents / 100).toFixed(2),
			interval: plan.interval === 'year' ? 'year' : 'month',
			features: plan.features.join('\n'),
			trial_days: String(plan.trial_days),
			stripe_price_id: plan.stripe_price_id ?? '',
			push_to_stripe_subscribers: false,
			rollout_audience: 'linked_subscriptions_only'
		};
	}

	function cancelEdit() {
		editingId = null;
		rolloutPreview = null;
		showRolloutConfirm = false;
	}

	async function fetchRolloutPreview(planId: string) {
		rolloutPreviewLoading = true;
		rolloutPreview = null;
		try {
			rolloutPreview = await api.get<RolloutPreview>(
				`/api/admin/pricing/plans/${planId}/rollout-preview?audience=${editDraft.rollout_audience}`
			);
		} catch {
			rolloutPreview = null;
		} finally {
			rolloutPreviewLoading = false;
		}
	}

	async function saveEdit(planId: string) {
		// If rollout is enabled and not yet confirmed, show confirmation first.
		if (editDraft.push_to_stripe_subscribers && !showRolloutConfirm) {
			await fetchRolloutPreview(planId);
			showRolloutConfirm = true;
			return;
		}

		saving = planId;
		lastStripeRolloutSummary = null;
		showRolloutConfirm = false;
		try {
			const payload: UpdatePricingPlanPayload = {
				name: editDraft.name,
				amount_cents: Math.round(parseFloat(editDraft.amount_dollars) * 100),
				interval: editDraft.interval,
				features: editDraft.features
					.split('\n')
					.map((f) => f.trim())
					.filter(Boolean),
				trial_days: parseInt(editDraft.trial_days) || 0,
				stripe_price_id: editDraft.stripe_price_id
			};
			if (editDraft.push_to_stripe_subscribers) {
				payload.stripe_rollout = {
					push_to_stripe_subscriptions: true,
					audience: editDraft.rollout_audience,
					skip_price_protected: true
				};
			}
			const fetchOpts = editDraft.push_to_stripe_subscribers
				? { headers: { 'Idempotency-Key': crypto.randomUUID() } }
				: undefined;
			const res = await api.put<AdminUpdatePricingPlanResponse>(
				`/api/admin/pricing/plans/${planId}`,
				payload,
				fetchOpts
			);
			if (res.stripe_rollout) {
				const r = res.stripe_rollout;
				const failPreview = r.failed
					.slice(0, 3)
					.map((f) => `${f.stripe_subscription_id}: ${f.error}`)
					.join(' · ');
				const more = r.failed.length > 3 ? ` (+${r.failed.length - 3} more)` : '';
				const skippedNote =
					r.skipped_grandfathered > 0
						? ` ${r.skipped_grandfathered} grandfathered member(s) kept their price.`
						: '';
				lastStripeRolloutSummary = `Stripe rollout: ${r.succeeded}/${r.targeted} updated.${skippedNote}${
					r.failed.length ? ` Failures: ${failPreview}${more}` : ''
				}`;
			}
			await loadPlans();
			await loadPriceLog();
			editingId = null;
			rolloutPreview = null;
		} catch {
			// keep form open on error
		} finally {
			saving = null;
		}
	}

	async function createPlan() {
		saving = 'new';
		try {
			const payload: CreatePricingPlanPayload = {
				name: newPlan.name,
				amount_cents: Math.round(parseFloat(newPlan.amount_dollars) * 100),
				interval: newPlan.interval,
				features: newPlan.features
					.split('\n')
					.map((f) => f.trim())
					.filter(Boolean),
				trial_days: parseInt(newPlan.trial_days) || 0,
				stripe_price_id: newPlan.stripe_price_id
			};
			await api.post('/api/admin/pricing/plans', payload);
			await loadPlans();
			showNewForm = false;
			newPlan = {
				name: '',
				amount_dollars: '',
				interval: 'month',
				features: '',
				trial_days: '0',
				stripe_price_id: ''
			};
		} catch {
			// keep form open on error
		} finally {
			saving = null;
		}
	}

	function formatMoney(cents: number): string {
		return (
			'$' +
			(cents / 100).toLocaleString('en-US', {
				minimumFractionDigits: 0,
				maximumFractionDigits: 0
			})
		);
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function intervalLabel(interval: string): string {
		return interval === 'month' ? '/mo' : '/yr';
	}
</script>

<svelte:head>
	<title>Pricing Plans - Admin</title>
</svelte:head>

<div class="plans-page">
	<div class="plans-page__header">
		<div class="plans-page__header-left">
			<a href={resolve('/admin/subscriptions')} class="plans-page__back">
				<ArrowLeftIcon size={18} weight="bold" />
			</a>
			<div>
				<h1 class="plans-page__title">Pricing Plans</h1>
				<p class="plans-page__subtitle">{plans.length} plans configured</p>
			</div>
		</div>
		<button
			class="plans-page__add-btn"
			onclick={() => {
				showNewForm = true;
				editingId = null;
			}}
		>
			<PlusIcon size={16} weight="bold" /> Add Plan
		</button>
	</div>

	{#if lastStripeRolloutSummary}
		<p class="rollout-banner" role="status">{lastStripeRolloutSummary}</p>
	{/if}

	{#if showNewForm}
		<div class="plan-card plan-card--form">
			<h3 class="plan-card__form-title">New Plan</h3>
			<div class="plan-card__fields">
				<label class="field" for="new-plan-name">
					<span class="field__label">Name</span>
					<input
						id="new-plan-name"
						name="plan-name"
						type="text"
						class="field__input"
						bind:value={newPlan.name}
						placeholder="e.g. Pro Monthly"
					/>
				</label>
				<div class="field-row">
					<label class="field field--half" for="new-plan-amount">
						<span class="field__label">Price ($)</span>
						<input
							id="new-plan-amount"
							name="plan-amount"
							type="text"
							class="field__input"
							bind:value={newPlan.amount_dollars}
							placeholder="49.00"
						/>
					</label>
					<label class="field field--half" for="new-plan-interval">
						<span class="field__label">Interval</span>
						<select
							id="new-plan-interval"
							name="plan-interval"
							class="field__input"
							bind:value={newPlan.interval}
						>
							<option value="month">Monthly</option>
							<option value="year">Yearly</option>
						</select>
					</label>
				</div>
				<label class="field" for="new-plan-features">
					<span class="field__label">Features (one per line)</span>
					<textarea
						id="new-plan-features"
						name="plan-features"
						class="field__textarea"
						bind:value={newPlan.features}
						rows="4"
						placeholder="Feature one&#10;Feature two"
					></textarea>
				</label>
				<div class="field-row">
					<label class="field field--half" for="new-plan-trial-days">
						<span class="field__label">Trial Days</span>
						<input
							id="new-plan-trial-days"
							name="plan-trial-days"
							type="number"
							class="field__input"
							bind:value={newPlan.trial_days}
						/>
					</label>
					<label class="field field--half" for="new-plan-stripe-price">
						<span class="field__label">Stripe Price ID</span>
						<input
							id="new-plan-stripe-price"
							name="plan-stripe-price-id"
							type="text"
							class="field__input"
							bind:value={newPlan.stripe_price_id}
							placeholder="price_..."
						/>
					</label>
				</div>
			</div>
			<div class="plan-card__actions">
				<button
					class="btn btn--ghost"
					onclick={() => {
						showNewForm = false;
					}}>Cancel</button
				>
				<button
					class="btn btn--primary"
					disabled={saving === 'new' || !newPlan.name || !newPlan.amount_dollars}
					onclick={createPlan}
				>
					{#if saving === 'new'}Saving...{:else}<FloppyDiskIcon size={15} weight="bold" /> Create{/if}
				</button>
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="plans-page__grid">
			{#each Array(3) as _, i (i)}
				<div class="plan-card plan-card--skeleton">
					<div class="skeleton-block skeleton-block--title"></div>
					<div class="skeleton-block skeleton-block--price"></div>
					<div class="skeleton-block skeleton-block--features"></div>
				</div>
			{/each}
		</div>
	{:else if plans.length === 0 && !showNewForm}
		<div class="plans-page__empty">
			<p>No pricing plans yet. Create your first plan to get started.</p>
		</div>
	{:else}
		<div class="plans-page__grid">
			{#each plans as plan (plan.id)}
				<div
					class="plan-card"
					class:plan-card--popular={plan.is_popular}
					class:plan-card--inactive={!plan.is_active}
				>
					{#if editingId === plan.id}
						<div class="plan-card__fields">
							<label class="field" for={`edit-plan-name-${plan.id}`}>
								<span class="field__label">Name</span>
								<input
									id={`edit-plan-name-${plan.id}`}
									name="plan-name"
									type="text"
									class="field__input"
									bind:value={editDraft.name}
								/>
							</label>
							<div class="field-row">
								<label
									class="field field--half"
									for={`edit-plan-amount-${plan.id}`}
								>
									<span class="field__label">Price ($)</span>
									<input
										id={`edit-plan-amount-${plan.id}`}
										name="plan-amount"
										type="text"
										class="field__input"
										bind:value={editDraft.amount_dollars}
									/>
								</label>
								<label
									class="field field--half"
									for={`edit-plan-interval-${plan.id}`}
								>
									<span class="field__label">Interval</span>
									<select
										id={`edit-plan-interval-${plan.id}`}
										name="plan-interval"
										class="field__input"
										bind:value={editDraft.interval}
									>
										<option value="month">Monthly</option>
										<option value="year">Yearly</option>
									</select>
								</label>
							</div>
							<label class="field" for={`edit-plan-features-${plan.id}`}>
								<span class="field__label">Features (one per line)</span>
								<textarea
									id={`edit-plan-features-${plan.id}`}
									name="plan-features"
									class="field__textarea"
									bind:value={editDraft.features}
									rows="4"
								></textarea>
							</label>
							<div class="field-row">
								<label
									class="field field--half"
									for={`edit-plan-trial-days-${plan.id}`}
								>
									<span class="field__label">Trial Days</span>
									<input
										id={`edit-plan-trial-days-${plan.id}`}
										name="plan-trial-days"
										type="number"
										class="field__input"
										bind:value={editDraft.trial_days}
									/>
								</label>
								<label
									class="field field--half"
									for={`edit-plan-stripe-price-${plan.id}`}
								>
									<span class="field__label">Stripe Price ID</span>
									<input
										id={`edit-plan-stripe-price-${plan.id}`}
										name="plan-stripe-price-id"
										type="text"
										class="field__input"
										bind:value={editDraft.stripe_price_id}
									/>
								</label>
							</div>
							<div class="rollout-panel">
								<label
									class="rollout-panel__toggle"
									for={`edit-plan-push-stripe-${plan.id}`}
								>
									<input
										id={`edit-plan-push-stripe-${plan.id}`}
										name="plan-push-stripe"
										type="checkbox"
										bind:checked={editDraft.push_to_stripe_subscribers}
										onchange={() => {
											rolloutPreview = null;
											showRolloutConfirm = false;
										}}
									/>
									<span>Also update existing Stripe subscriptions after save</span
									>
								</label>
								<p class="rollout-panel__hint">
									Requires Stripe configuration and an <code>Idempotency-Key</code
									> (sent automatically). Only runs when you change billing fields (price,
									currency, interval, or Stripe price id). Members with grandfather
									price protection are always skipped.
								</p>
								{#if editDraft.push_to_stripe_subscribers}
									<fieldset class="rollout-panel__audience">
										<legend class="rollout-panel__legend"
											>Which subscriptions to update</legend
										>
										<label class="rollout-panel__radio">
											<input
												type="radio"
												name="rollout-audience-{plan.id}"
												value="linked_subscriptions_only"
												bind:group={editDraft.rollout_audience}
												onchange={() => {
													rolloutPreview = null;
													showRolloutConfirm = false;
												}}
											/>
											<span
												>Linked only (recommended) — members who checked out
												with this catalog plan</span
											>
										</label>
										<label class="rollout-panel__radio">
											<input
												type="radio"
												name="rollout-audience-{plan.id}"
												value="linked_and_unlinked_legacy_same_cadence"
												bind:group={editDraft.rollout_audience}
												onchange={() => {
													rolloutPreview = null;
													showRolloutConfirm = false;
												}}
											/>
											<span
												>Linked + legacy same cadence — also monthly/annual
												rows missing catalog link (use with care)</span
											>
										</label>
									</fieldset>

									{#if showRolloutConfirm && rolloutPreview}
										<div class="rollout-confirm" role="alert">
											<p class="rollout-confirm__heading">
												Confirm Stripe price update
											</p>
											<ul class="rollout-confirm__counts">
												<li>
													<strong>{rolloutPreview.would_update}</strong> subscription(s)
													will be updated to the new price
												</li>
												{#if rolloutPreview.would_skip_grandfathered > 0}
													<li>
														<strong
															>{rolloutPreview.would_skip_grandfathered}</strong
														> grandfathered member(s) will keep their current price
													</li>
												{/if}
											</ul>
											<p class="rollout-confirm__warning">
												This cannot be undone from the admin. Are you sure?
											</p>
										</div>
									{:else if rolloutPreviewLoading}
										<p class="rollout-panel__hint">Loading preview…</p>
									{/if}
								{/if}
							</div>
						</div>
						<div class="plan-card__actions">
							<button
								class="btn btn--ghost"
								onclick={() => {
									if (showRolloutConfirm) {
										showRolloutConfirm = false;
										rolloutPreview = null;
									} else {
										cancelEdit();
									}
								}}
							>
								{showRolloutConfirm ? 'Back' : 'Cancel'}
							</button>
							<button
								class="btn btn--primary"
								disabled={saving === plan.id || rolloutPreviewLoading}
								onclick={() => saveEdit(plan.id)}
							>
								{#if saving === plan.id}
									Saving…
								{:else if showRolloutConfirm}
									Confirm &amp; Push to Stripe
								{:else}
									<FloppyDiskIcon size={15} weight="bold" /> Save
								{/if}
							</button>
						</div>
					{:else}
						<div class="plan-card__top">
							<div class="plan-card__badges">
								{#if plan.is_active}
									<span class="badge badge--active">Active</span>
								{:else}
									<span class="badge badge--inactive">Inactive</span>
								{/if}
								{#if plan.is_popular}
									<span class="badge badge--popular"
										><StarIcon size={12} weight="fill" /> Popular</span
									>
								{/if}
							</div>
							<button class="plan-card__edit-btn" onclick={() => startEdit(plan)}>
								<PencilSimpleIcon size={15} weight="bold" /> Edit
							</button>
						</div>
						<h3 class="plan-card__name">{plan.name}</h3>
						<div class="plan-card__price">
							<span class="plan-card__amount">{formatMoney(plan.amount_cents)}</span>
							<span class="plan-card__interval">{intervalLabel(plan.interval)}</span>
						</div>
						{#if plan.trial_days > 0}
							<p class="plan-card__trial">{plan.trial_days}-day free trial</p>
						{/if}
						<ul class="plan-card__features">
							{#each plan.features as feature, fi (`${plan.id}-${fi}-${feature}`)}
								<li class="plan-card__feature">
									<CheckCircleIcon size={15} weight="fill" />
									<span>{feature}</span>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/each}
		</div>
	{/if}

	<div class="log-section">
		<button
			class="log-section__toggle"
			onclick={() => {
				logOpen = !logOpen;
			}}
		>
			<span class="log-section__toggle-text">Price Change Log</span>
			{#if logOpen}<CaretUpIcon size={16} weight="bold" />{:else}<CaretDownIcon
					size={16}
					weight="bold"
				/>{/if}
		</button>
		{#if logOpen}
			<div class="log-section__content">
				{#if priceLog.length === 0}
					<p class="log-section__empty">No price changes recorded.</p>
				{:else}
					<div class="log-section__list">
						{#each priceLog as entry (entry.id)}
							<div class="log-entry">
								<div class="log-entry__info">
									<span class="log-entry__plan">{entry.plan_name}</span>
									<span class="log-entry__change">
										{formatMoney(entry.old_amount_cents)} &rarr; {formatMoney(
											entry.new_amount_cents
										)}
									</span>
								</div>
								<div class="log-entry__meta">
									<span>{entry.changed_by}</span>
									<span class="log-entry__date"
										>{formatDate(entry.changed_at)}</span
									>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.plans-page__header {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}
	.plans-page__header-left {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	.plans-page__back {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-2xl);
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-grey-300);
		text-decoration: none;
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.plans-page__back:hover {
		border-color: var(--color-teal);
		color: var(--color-white);
	}
	.plans-page__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.plans-page__subtitle {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-top: 0.1rem;
	}
	.plans-page__add-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		align-self: flex-start;
		padding: 0.55rem 1rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		border: none;
		border-radius: var(--radius-2xl);
		cursor: pointer;
		transition:
			opacity var(--duration-200) var(--ease-out),
			transform var(--duration-200) var(--ease-out);
	}
	.plans-page__add-btn:hover {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	.rollout-banner {
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
		background: rgba(15, 164, 175, 0.08);
		border: 1px solid rgba(15, 164, 175, 0.25);
		border-radius: var(--radius-2xl);
		padding: 0.75rem 1rem;
		margin-bottom: 1rem;
	}

	.rollout-panel {
		margin-top: 0.5rem;
		padding: 0.75rem;
		border-radius: var(--radius-md);
		background: rgba(0, 0, 0, 0.15);
		border: 1px solid rgba(255, 255, 255, 0.06);
	}
	.rollout-panel__toggle {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
		cursor: pointer;
	}
	.rollout-panel__toggle input {
		margin-top: 0.2rem;
	}
	.rollout-panel__hint {
		font-size: var(--fs-2xs);
		color: var(--color-grey-500);
		margin: 0.4rem 0 0 1.5rem;
		line-height: 1.4;
	}
	.rollout-panel__hint code {
		font-size: 0.85em;
		color: var(--color-teal);
	}
	.rollout-panel__audience {
		border: none;
		margin: 0.75rem 0 0;
		padding: 0 0 0 1.5rem;
	}
	.rollout-panel__legend {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-400);
		padding: 0;
		margin-bottom: 0.35rem;
	}
	.rollout-panel__radio {
		display: flex;
		gap: 0.45rem;
		align-items: flex-start;
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
		margin-top: 0.35rem;
		cursor: pointer;
	}
	.rollout-panel__radio input {
		margin-top: 0.15rem;
	}

	.rollout-confirm {
		margin-top: 0.75rem;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-md);
		background: rgba(255, 180, 0, 0.06);
		border: 1px solid rgba(255, 180, 0, 0.25);
	}
	.rollout-confirm__heading {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-warning, #f5a623);
		margin-bottom: 0.4rem;
	}
	.rollout-confirm__counts {
		margin: 0 0 0.4rem 1rem;
		padding: 0;
		font-size: var(--fs-xs);
		color: var(--color-grey-200);
		line-height: 1.6;
	}
	.rollout-confirm__warning {
		font-size: var(--fs-2xs);
		color: var(--color-grey-400);
	}

	/* Grid */
	.plans-page__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1rem;
		margin-bottom: 2rem;
	}
	.plans-page__empty {
		text-align: center;
		padding: 3rem 1rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
	}

	/* Card */
	.plan-card {
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.plan-card--popular {
		border-color: rgba(212, 168, 67, 0.3);
	}
	.plan-card--inactive {
		opacity: 0.6;
	}
	.plan-card--form {
		border-color: rgba(15, 164, 175, 0.3);
		margin-bottom: 1rem;
	}
	.plan-card--skeleton {
		min-height: 14rem;
	}

	.skeleton-block {
		border-radius: var(--radius-md);
		background: rgba(255, 255, 255, 0.06);
		animation: shimmer 1.5s infinite;
	}
	.skeleton-block--title {
		height: 1.25rem;
		width: 50%;
		margin-bottom: 0.75rem;
	}
	.skeleton-block--price {
		height: 2rem;
		width: 35%;
		margin-bottom: 1rem;
	}
	.skeleton-block--features {
		height: 5rem;
		width: 100%;
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

	.plan-card__top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
	}
	.plan-card__badges {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
	}
	.plan-card__form-title {
		font-size: var(--fs-md);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin-bottom: 1rem;
	}

	.badge {
		font-size: var(--fs-2xs);
		font-weight: var(--w-semibold);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
	}
	.badge--active {
		background-color: rgba(34, 181, 115, 0.12);
		color: var(--color-green);
	}
	.badge--inactive {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
	.badge--popular {
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		background-color: rgba(212, 168, 67, 0.15);
		color: var(--color-gold);
	}

	.plan-card__edit-btn {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.3rem 0.6rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
		background: none;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			border-color var(--duration-200) var(--ease-out),
			color var(--duration-200) var(--ease-out);
	}
	.plan-card__edit-btn:hover {
		border-color: var(--color-teal);
		color: var(--color-white);
	}

	.plan-card__name {
		font-size: var(--fs-md);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin-bottom: 0.35rem;
	}
	.plan-card__price {
		display: flex;
		align-items: baseline;
		gap: 0.15rem;
		margin-bottom: 0.4rem;
	}
	.plan-card__amount {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}
	.plan-card__interval {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
	}
	.plan-card__trial {
		font-size: var(--fs-xs);
		color: var(--color-teal);
		margin-bottom: 0.5rem;
	}

	.plan-card__features {
		list-style: none;
		padding: 0;
		margin: 0.5rem 0 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.plan-card__feature {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}
	.plan-card__feature :global(svg) {
		color: var(--color-green);
		flex-shrink: 0;
	}

	/* Form fields */
	.plan-card__fields {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.plan-card__actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 1rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.field__label {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
	}
	.field__input {
		padding: 0.55rem 0.75rem;
		background-color: var(--color-navy);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: var(--font-ui);
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.field__textarea {
		padding: 0.55rem 0.75rem;
		background-color: var(--color-navy);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: var(--font-ui);
		resize: vertical;
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.field__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.field-row {
		display: flex;
		gap: 0.75rem;
	}
	.field--half {
		flex: 1;
	}

	/* Buttons */
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.5rem 0.85rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		border-radius: var(--radius-md);
		cursor: pointer;
		border: none;
		transition: opacity var(--duration-200) var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
	}
	.btn--primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn--ghost {
		background: none;
		color: var(--color-grey-400);
		border: 1px solid rgba(255, 255, 255, 0.1);
	}
	.btn--ghost:hover:not(:disabled) {
		color: var(--color-white);
		border-color: rgba(255, 255, 255, 0.2);
	}

	/* Price Change Log */
	.log-section {
		margin-top: 1rem;
	}
	.log-section__toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 0.85rem 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: border-color var(--duration-200) var(--ease-out);
	}
	.log-section__toggle:hover {
		border-color: rgba(255, 255, 255, 0.12);
	}
	.log-section__toggle-text {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	.log-section__content {
		margin-top: -1px;
		padding: 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 0 0 var(--radius-xl) var(--radius-xl);
	}
	.log-section__empty {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		text-align: center;
		padding: 1rem 0;
	}
	.log-section__list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.log-entry {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 0.6rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.log-entry:last-child {
		border-bottom: none;
	}
	.log-entry__info {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.log-entry__plan {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}
	.log-entry__change {
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}
	.log-entry__meta {
		display: flex;
		justify-content: space-between;
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
	}
	.log-entry__date {
		color: var(--color-grey-500);
	}

	/* Responsive */
	@media (min-width: 480px) {
		.plans-page__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (min-width: 768px) {
		.plans-page__header {
			flex-direction: row;
			justify-content: space-between;
			align-items: flex-start;
		}
		.plans-page__title {
			font-size: var(--fs-2xl);
		}
		.plans-page__add-btn {
			align-self: auto;
			padding: 0.6rem 1.25rem;
			font-size: var(--fs-sm);
		}
		.plan-card {
			padding: 1.5rem;
		}
		.log-entry {
			flex-direction: row;
			justify-content: space-between;
			align-items: center;
		}
		.log-entry__meta {
			gap: 1rem;
		}
	}
	@media (min-width: 1024px) {
		.plans-page__grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}
</style>
