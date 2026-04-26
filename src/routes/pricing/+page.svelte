<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import type { PricingPlan } from '$lib/api/types';
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';
	import CheckIcon from 'phosphor-svelte/lib/CheckIcon';
	import TagIcon from 'phosphor-svelte/lib/TagIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import {
		PRICING_ANNUAL_SAVINGS_USD,
		PRICING_ANNUAL_SAVINGS_PERCENT_ROUNDED
	} from '$lib/data/pricing';

	interface DisplayPlan {
		id: string;
		name: string;
		slug: string;
		amount_cents: number;
		currency: string;
		interval: string;
		features: string[];
		is_popular: boolean;
		highlight_text: string | null;
		trial_days: number;
		stripe_price_id: string | null;
	}

	let plans = $state<DisplayPlan[]>([]);
	let loading = $state(true);
	let billingCycle = $state<'month' | 'year'>('month');

	// Coupon
	let couponCode = $state('');
	let couponOpen = $state(false);
	let couponLoading = $state(false);
	let couponError = $state('');
	let couponValid = $state(false);
	let discountPercent = $state(0);
	let discountMessage = $state('');

	// FAQ
	let openFaq = $state(-1);

	const faqs = [
		{
			q: 'Can I cancel anytime?',
			a: 'Yes. Cancel any time from your account settings. Your access stays active through the end of your current billing period.'
		},
		{
			q: 'What payment methods do you accept?',
			a: 'We process payments securely through Stripe and accept major credit and debit cards, plus Apple Pay and Google Pay where supported.'
		},
		{
			q: 'Is there a free trial?',
			a: 'Some plans include a trial window. Trial availability is shown directly on each plan card before checkout.'
		},
		{
			q: 'Can I switch between monthly and annual?',
			a: 'Yes. You can change plans at any time. Billing is adjusted automatically, including prorated credits when applicable.'
		},
		{
			q: 'What happens after I subscribe?',
			a: 'You get immediate member access to course lessons, weekly watchlists, and alert channels, plus onboarding guidance to start using the workflow quickly.'
		}
	];

	const jsonLd = buildJsonLd([
		webPageSchema({
			path: '/pricing',
			title: 'Pricing Plans - Precision Options Signals',
			description:
				'Compare monthly and annual plans for Precision Options Signals. Access weekly watchlists, course training, and trade execution guidance.',
			speakable: '.pricing-page__title, .pricing-page__subtitle'
		})
	]);

	const fallbackPlans: DisplayPlan[] = [
		{
			id: '1',
			name: 'Monthly',
			slug: 'monthly',
			amount_cents: 4900,
			currency: 'usd',
			interval: 'month',
			features: [
				'Weekly watchlists & trade alerts',
				'Full course library access',
				'Members-only community',
				'Mobile app access'
			],
			is_popular: false,
			highlight_text: null,
			trial_days: 0,
			stripe_price_id: null
		},
		{
			id: '2',
			name: 'Annual',
			slug: 'annual',
			amount_cents: 39900,
			currency: 'usd',
			interval: 'year',
			features: [
				'Everything in Monthly',
				`Save $${PRICING_ANNUAL_SAVINGS_USD}/year vs monthly`,
				'Priority support',
				'Exclusive annual member content'
			],
			is_popular: true,
			highlight_text: 'Best Value',
			trial_days: 0,
			stripe_price_id: null
		}
	];

	let filteredPlans = $derived(
		plans.filter((p) =>
			billingCycle === 'month' ? p.interval === 'month' : p.interval === 'year'
		)
	);

	onMount(async () => {
		try {
			const res = await api.get<PricingPlan[]>('/api/pricing/plans');
			plans = res.map((p) => ({
				id: p.id,
				name: p.name,
				slug: p.slug,
				amount_cents: p.amount_cents,
				currency: p.currency,
				interval: p.interval,
				features: Array.isArray(p.features) ? p.features : [],
				is_popular: p.is_popular,
				highlight_text: p.highlight_text,
				trial_days: p.trial_days,
				stripe_price_id: p.stripe_price_id
			}));
		} catch {
			plans = fallbackPlans;
		} finally {
			loading = false;
		}
	});

	function formatPrice(cents: number): string {
		return `$${(cents / 100).toFixed(0)}`;
	}

	function discountedPrice(cents: number): number {
		if (!couponValid || discountPercent <= 0) return cents;
		return Math.round(cents * (1 - discountPercent / 100));
	}

	async function validateCoupon() {
		if (!couponCode.trim()) return;
		couponLoading = true;
		couponError = '';
		couponValid = false;
		try {
			const res = await api.post<{
				valid: boolean;
				discount_amount_cents: number | null;
				message: string;
			}>('/api/coupons/validate', { code: couponCode.trim() });
			if (res.valid) {
				couponValid = true;
				discountPercent = res.discount_amount_cents ? 0 : 20; // fallback
				discountMessage = res.message;
			} else {
				couponError = res.message;
			}
		} catch {
			couponError = 'Could not validate coupon. Try again.';
		} finally {
			couponLoading = false;
		}
	}

	function getCheckoutUrl(plan: DisplayPlan): string {
		const base =
			plan.interval === 'month' ? resolve('/pricing/monthly') : resolve('/pricing/annual');
		return couponValid ? `${base}?coupon=${encodeURIComponent(couponCode)}` : base;
	}
</script>

<Seo
	title="Pricing Plans - Precision Options Signals"
	description="Compare monthly and annual plans for Precision Options Signals. Access weekly watchlists, course training, and trade execution guidance."
	ogTitle="Precision Options Signals Pricing - Monthly and Annual"
	{jsonLd}
/>

<div class="pricing-page">
	<div class="pricing-page__header">
		<h1 class="pricing-page__title">Choose the Plan That Fits Your Trading Workflow</h1>
		<p class="pricing-page__subtitle">
			Compare monthly and annual options for structured training, weekly watchlists, and
			disciplined execution support.
		</p>

		<div class="pricing-page__toggle">
			<button
				class="toggle-btn"
				class:active={billingCycle === 'month'}
				onclick={() => (billingCycle = 'month')}>Monthly</button
			>
			<button
				class="toggle-btn"
				class:active={billingCycle === 'year'}
				onclick={() => (billingCycle = 'year')}
				>Annual <span class="toggle-badge"
					>Save {PRICING_ANNUAL_SAVINGS_PERCENT_ROUNDED}%</span
				></button
			>
		</div>
	</div>

	{#if loading}
		<div class="pricing-page__loading">Loading plans...</div>
	{:else}
		<div class="pricing-page__cards">
			{#each filteredPlans as plan (plan.id)}
				<div class="plan-card" class:plan-card--popular={plan.is_popular}>
					{#if plan.highlight_text}
						<div class="plan-card__badge">{plan.highlight_text}</div>
					{/if}
					<h2 class="plan-card__name">{plan.name}</h2>
					<div class="plan-card__price">
						{#if couponValid}
							<span class="plan-card__price-old"
								>{formatPrice(plan.amount_cents)}</span
							>
							<span class="plan-card__price-new"
								>{formatPrice(discountedPrice(plan.amount_cents))}</span
							>
						{:else}
							<span class="plan-card__price-amount"
								>{formatPrice(plan.amount_cents)}</span
							>
						{/if}
						<span class="plan-card__price-suffix"
							>/{plan.interval === 'month' ? 'mo' : 'yr'}</span
						>
					</div>
					{#if plan.interval === 'year'}
						<p class="plan-card__monthly-eq">
							That's {formatPrice(Math.round(plan.amount_cents / 12))}/mo
						</p>
					{/if}
					{#if plan.trial_days > 0}
						<p class="plan-card__trial">{plan.trial_days}-day free trial</p>
					{/if}
					<ul class="plan-card__features">
						{#each plan.features as feature (feature)}
							<li><CheckIcon size={16} weight="bold" /> {feature}</li>
						{/each}
					</ul>
					<!-- eslint-disable svelte/no-navigation-without-resolve -- href is built by getCheckoutUrl() which uses resolve() internally; the lint rule cannot see through the helper -->
					<a
						href={getCheckoutUrl(plan)}
						class="plan-card__cta"
						class:plan-card__cta--primary={plan.is_popular}
					>
						Get Started
					</a>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				</div>
			{/each}
		</div>

		<!-- Coupon Section -->
		<div class="coupon-section">
			<button class="coupon-toggle" onclick={() => (couponOpen = !couponOpen)}>
				<TagIcon size={18} /> Have a coupon code?
				<CaretDownIcon size={14} class={couponOpen ? 'rotate' : ''} />
			</button>
			{#if couponOpen}
				<div class="coupon-form">
					<input
						type="text"
						bind:value={couponCode}
						placeholder="Enter coupon code"
						class="coupon-input"
					/>
					<button onclick={validateCoupon} disabled={couponLoading} class="coupon-btn">
						{couponLoading ? 'Validating...' : 'Apply'}
					</button>
				</div>
				{#if couponError}<p class="coupon-error">{couponError}</p>{/if}
				{#if couponValid}<p class="coupon-success">{discountMessage}</p>{/if}
			{/if}
		</div>

		<!-- FAQ -->
		<div class="faq-section">
			<h2 class="faq-title">Frequently Asked Questions</h2>
			{#each faqs as faq, i (faq.q)}
				<button class="faq-item" onclick={() => (openFaq = openFaq === i ? -1 : i)}>
					<div class="faq-q">
						{faq.q}
						<CaretDownIcon size={16} class={openFaq === i ? 'rotate' : ''} />
					</div>
					{#if openFaq === i}
						<p class="faq-a">{faq.a}</p>
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.pricing-page {
		max-width: 900px;
		margin: 0 auto;
		padding: 3rem 1rem;
	}
	.pricing-page__header {
		text-align: center;
		margin-bottom: 3rem;
	}
	.pricing-page__title {
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.75rem;
	}
	.pricing-page__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-lg);
		max-width: 600px;
		margin: 0 auto 2rem;
	}
	.pricing-page__toggle {
		display: inline-flex;
		background: rgba(255, 255, 255, 0.05);
		border-radius: var(--radius-full);
		padding: 0.25rem;
		gap: 0.25rem;
	}
	.toggle-btn {
		padding: 0.6rem 1.5rem;
		border-radius: var(--radius-full);
		border: none;
		background: transparent;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: all 200ms;
	}
	.toggle-btn.active {
		background: var(--color-teal);
		color: var(--color-white);
	}
	.toggle-badge {
		font-size: var(--fs-xs);
		background: rgba(255, 255, 255, 0.2);
		padding: 0.15rem 0.4rem;
		border-radius: var(--radius-full);
		margin-left: 0.35rem;
	}
	.pricing-page__loading {
		text-align: center;
		color: var(--color-grey-400);
		padding: 4rem 0;
	}
	.pricing-page__cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 1.5rem;
		margin-bottom: 3rem;
	}
	.plan-card {
		position: relative;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 2rem;
		transition:
			transform 200ms,
			border-color 200ms;
	}
	.plan-card:hover {
		transform: translateY(-4px);
	}
	.plan-card--popular {
		border-color: var(--color-teal);
		background: rgba(15, 164, 175, 0.05);
	}
	.plan-card__badge {
		position: absolute;
		top: -0.75rem;
		left: 50%;
		transform: translateX(-50%);
		background: var(--color-teal);
		color: var(--color-white);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		padding: 0.25rem 1rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.plan-card__name {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 1rem;
	}
	.plan-card__price {
		display: flex;
		align-items: baseline;
		gap: 0.25rem;
		margin-bottom: 0.5rem;
	}
	.plan-card__price-amount,
	.plan-card__price-new {
		font-size: 3rem;
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1;
	}
	.plan-card__price-old {
		font-size: 1.5rem;
		color: var(--color-grey-500);
		text-decoration: line-through;
		margin-right: 0.5rem;
	}
	.plan-card__price-new {
		color: #34d399;
	}
	.plan-card__price-suffix {
		font-size: var(--fs-lg);
		color: var(--color-grey-400);
	}
	.plan-card__monthly-eq {
		font-size: var(--fs-sm);
		color: var(--color-teal);
		margin-bottom: 0.5rem;
	}
	.plan-card__trial {
		font-size: var(--fs-sm);
		color: #34d399;
		font-weight: var(--w-medium);
		margin-bottom: 0.5rem;
	}
	.plan-card__features {
		list-style: none;
		padding: 0;
		margin: 1.5rem 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.plan-card__features li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
	}
	.plan-card__features li :global(svg) {
		color: var(--color-teal);
		flex-shrink: 0;
	}
	.plan-card__cta {
		display: block;
		text-align: center;
		padding: 0.85rem;
		border-radius: var(--radius-lg);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		text-decoration: none;
		transition: all 200ms;
		border: 1px solid rgba(255, 255, 255, 0.15);
		color: var(--color-white);
		background: transparent;
	}
	.plan-card__cta:hover {
		background: rgba(255, 255, 255, 0.05);
	}
	.plan-card__cta--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		border-color: transparent;
	}
	.plan-card__cta--primary:hover {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	.coupon-section {
		max-width: 500px;
		margin: 0 auto 3rem;
		text-align: center;
	}
	.coupon-toggle {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		background: none;
		border: none;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
		padding: 0.5rem;
	}
	.coupon-toggle :global(.rotate) {
		transform: rotate(180deg);
	}
	.coupon-form {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
	}
	.coupon-input {
		flex: 1;
		padding: 0.65rem 1rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: monospace;
	}
	.coupon-input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.coupon-btn {
		padding: 0.65rem 1.25rem;
		background: var(--color-teal);
		color: var(--color-white);
		border: none;
		border-radius: var(--radius-lg);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		cursor: pointer;
	}
	.coupon-btn:disabled {
		opacity: 0.5;
	}
	.coupon-error {
		color: #fca5a5;
		font-size: var(--fs-sm);
		margin-top: 0.5rem;
	}
	.coupon-success {
		color: #34d399;
		font-size: var(--fs-sm);
		margin-top: 0.5rem;
	}

	.faq-section {
		max-width: 700px;
		margin: 0 auto;
	}
	.faq-title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		text-align: center;
		margin-bottom: 2rem;
	}
	.faq-item {
		display: block;
		width: 100%;
		text-align: left;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
		padding: 1.25rem;
		margin-bottom: 0.75rem;
		cursor: pointer;
		color: var(--color-white);
	}
	.faq-q {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
	}
	.faq-q :global(.rotate) {
		transform: rotate(180deg);
	}
	.faq-a {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		margin-top: 1rem;
		line-height: 1.6;
	}
</style>
