<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicCascade, EASE, DURATION } from '$lib/utils/animations';
	import { env } from '$env/dynamic/public';
	import { createCheckoutSession } from '$lib/utils/stripe';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { productSchema, buildJsonLd } from '$lib/seo/jsonld';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Sparkle from 'phosphor-svelte/lib/Sparkle';

	const jsonLd = buildJsonLd([
		productSchema({
			name: 'Explosive Swings Annual Plan',
			description:
				'Weekly options watchlists with 5-7 high-probability setups. Save $232/year vs monthly plan.',
			price: '932',
			path: '/pricing/annual',
			billingPeriod: 'year'
		})
	]);

	let heroRef: HTMLElement | undefined = $state();
	let isLoading = $state(false);

	const annualPriceId = env.PUBLIC_STRIPE_ANNUAL_PRICE_ID || '';

	onMount(() => {
		if (!heroRef) return;

		const ctx = gsap.context(() => {
			createCinematicCascade(heroRef!, [
				{
					selector: '.price-badge',
					duration: DURATION.fast,
					ease: EASE.snappy,
					y: 20,
					blur: 6,
					scale: 0.9,
					overlap: 0
				},
				{
					selector: '.price-title',
					duration: DURATION.cinematic,
					ease: EASE.cinematic,
					y: 36,
					blur: 10,
					scale: 0.95,
					overlap: 0.6
				},
				{
					selector: '.price-amount',
					duration: DURATION.slow,
					ease: EASE.cinematic,
					y: 30,
					blur: 8,
					scale: 0.96,
					overlap: 0.6
				},
				{
					selector: '.price-features',
					duration: DURATION.slow,
					ease: EASE.soft,
					y: 32,
					blur: 6,
					scale: 0.97,
					overlap: 0.55
				},
				{
					selector: '.price-cta',
					duration: DURATION.normal,
					ease: EASE.snappy,
					y: 24,
					blur: 6,
					scale: 0.96,
					overlap: 0.5
				}
			]);
		}, heroRef as HTMLElement);

		return () => ctx.revert();
	});

	async function handleCheckout() {
		isLoading = true;
		try {
			await createCheckoutSession(annualPriceId);
		} catch (error) {
			console.error('Checkout failed:', error);
			alert('Failed to start checkout. Please try again.');
			isLoading = false;
		}
	}

	const features = [
		'Weekly Sunday night watchlist (5–7 setups)',
		'Entry zones, profit targets, and stop losses',
		'SMS & email alerts for every trade',
		'Access to private Discord community',
		'Exclusive educational content & webinars',
		'Priority support & early access to new features',
		'Save $232/year vs monthly plan'
	];
</script>

<Seo
	title="Annual Plan - Explosive Swings"
	description="Get weekly options watchlists for $932/year. Save 20% vs monthly. 5-7 high-probability setups with entries, targets, and stops."
	ogTitle="Annual Plan $932/yr (Save 20%) - Explosive Swings"
	{jsonLd}
/>

<!-- Hero -->
<section bind:this={heroRef} class="page-hero">
	<div class="page-hero__grid-overlay"></div>

	<div class="page-hero__inner">
		<div class="price-badge page-badge page-badge--gold">
			<Sparkle size={18} weight="duotone" color="#D4A843" />
			<span class="page-badge__text page-badge__text--gold">Best Value</span>
		</div>

		<h1 class="price-title page-hero__title">Annual Plan -- Save 20%</h1>

		<div class="price-amount price-hero__amount">
			<div class="price-hero__price-row">
				<span class="price-hero__price">$932</span>
				<span class="price-hero__suffix">/year</span>
			</div>
			<div class="price-hero__savings-badge">
				<CheckCircle size={16} weight="fill" color="#22B573" />
				<span class="price-hero__savings-text">Save $232 vs monthly</span>
			</div>
		</div>
	</div>
</section>

<!-- Features -->
<section class="page-section page-section--white">
	<div class="page-container">
		<ScrollReveal>
			<div class="price-features page-narrow">
				<h2 class="page-section__heading page-section__heading--center">Everything Included</h2>

				<div class="feature-list">
					{#each features as feature, i}
						<div class="reveal-item feature-list__item" style="transition-delay: {i * 0.06}s">
							<CheckCircle size={24} weight="fill" color="#0FA4AF" class="feature-list__icon" />
							<p class="feature-list__text">{feature}</p>
						</div>
					{/each}
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Comparison -->
<section class="page-section page-section--off-white">
	<div class="page-container">
		<ScrollReveal>
			<div class="reveal-item comparison-card">
				<div class="comparison-card__header">
					<h3 class="comparison-card__header-title">Why Annual is Better</h3>
				</div>
				<div class="comparison-card__body">
					<div class="comparison-card__row">
						<span class="comparison-card__label">Monthly Plan (12 months)</span>
						<span class="comparison-card__value">$1,164</span>
					</div>
					<div class="comparison-card__row">
						<span class="comparison-card__label">Annual Plan</span>
						<span class="comparison-card__value comparison-card__value--teal">$932</span>
					</div>
					<div class="comparison-card__row">
						<span class="comparison-card__label comparison-card__label--green">You Save</span>
						<span class="comparison-card__value comparison-card__value--green-lg">$232</span>
					</div>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- CTA -->
<section class="page-section page-section--white">
	<div class="page-container page-container--center">
		<ScrollReveal>
			<div class="price-cta page-narrow">
				<h2 class="reveal-item page-section__heading page-section__heading--center">
					Lock In Your Savings Today
				</h2>

				<p class="reveal-item page-cta__desc">
					Get 12 months of weekly watchlists and save over $230 compared to paying monthly.
				</p>

				<div class="reveal-item page-cta__actions">
					<button onclick={handleCheckout} disabled={isLoading} class="page-cta-btn">
						{#if isLoading}
							Processing...
						{:else}
							Start Annual Plan -- $932/year
							<ArrowRight size={18} weight="bold" />
						{/if}
					</button>

					<a href="/pricing/monthly" class="page-cta__link">
						View Monthly Plan
						<ArrowRight size={14} weight="bold" />
					</a>
				</div>

				<p class="reveal-item page-cta__fine-print">
					Billed annually. All plans include a 30-day money-back guarantee.
				</p>
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	.page-hero {
		position: relative;
		overflow: hidden;
		padding-top: 4rem;
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.page-hero__grid-overlay {
		position: absolute;
		inset: 0;
		opacity: 0.02;
		background-image:
			linear-gradient(to right, white 1px, transparent 1px),
			linear-gradient(to bottom, white 1px, transparent 1px);
		background-size: 60px 60px;
	}

	.page-hero__inner {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 5rem 1rem;
		text-align: center;
	}

	@media (min-width: 640px) {
		.page-hero__inner {
			padding: 5rem 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-hero__inner {
			padding: 8rem 2rem;
		}
	}

	.page-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-full);
		padding: 0.5rem 1rem;
		margin-bottom: 1.5rem;
	}

	.page-badge--gold {
		border: 1px solid rgba(212, 168, 67, 0.3);
		background-color: rgba(212, 168, 67, 0.1);
	}

	.page-badge__text {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.page-badge__text--gold {
		color: var(--color-gold-light);
	}

	.page-hero__title {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.15;
		margin-bottom: 1.5rem;
		max-width: 48rem;
		margin-left: auto;
		margin-right: auto;
	}

	@media (min-width: 640px) {
		.page-hero__title {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 768px) {
		.page-hero__title {
			font-size: clamp(2.5rem, 5vw, 3rem);
		}
	}
	@media (min-width: 1024px) {
		.page-hero__title {
			font-size: clamp(3rem, 5vw, 3.75rem);
		}
	}

	.price-hero__amount {
		margin-bottom: 2rem;
	}

	.price-hero__price-row {
		display: flex;
		align-items: baseline;
		justify-content: center;
		gap: 0.5rem;
	}

	.price-hero__price {
		font-family: var(--font-heading);
		font-size: clamp(3rem, 6vw, 3.75rem);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.price-hero__suffix {
		color: var(--color-grey-300);
		font-size: var(--fs-xl);
	}

	.price-hero__savings-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.75rem;
		border-radius: var(--radius-lg);
		background-color: rgba(34, 181, 115, 0.1);
		padding: 0.5rem 1rem;
	}

	.price-hero__savings-text {
		color: var(--color-green);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
	}

	/* Sections */
	.page-section {
		padding: 4rem 0;
	}
	@media (min-width: 640px) {
		.page-section {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.page-section {
			padding: 7rem 0;
		}
	}

	.page-section--white {
		background-color: var(--color-white);
	}
	.page-section--off-white {
		background-color: var(--color-off-white);
	}

	.page-container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.page-container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-container {
			padding: 0 2rem;
		}
	}

	.page-container--center {
		text-align: center;
	}
	.page-narrow {
		max-width: 42rem;
		margin: 0 auto;
	}

	.page-section__heading {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		margin-bottom: 2.5rem;
	}

	@media (min-width: 640px) {
		.page-section__heading {
			font-size: var(--fs-3xl);
		}
	}
	@media (min-width: 768px) {
		.page-section__heading {
			font-size: var(--fs-4xl);
		}
	}

	.page-section__heading--center {
		text-align: center;
	}

	/* Feature list */
	.feature-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.feature-list__item {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
		border: 1px solid rgba(216, 220, 228, 0.8);
		background-color: var(--color-off-white);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
	}

	:global(.feature-list__icon) {
		flex-shrink: 0;
		margin-top: 0.125rem;
	}

	.feature-list__text {
		color: var(--color-grey-800);
		line-height: 1.65;
	}

	/* Comparison card */
	.comparison-card {
		max-width: 42rem;
		margin: 0 auto;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		border: 1px solid var(--color-grey-200);
		background-color: var(--color-white);
	}

	.comparison-card__header {
		background: linear-gradient(to right, var(--color-teal), var(--color-teal-light));
		padding: 1.5rem;
		text-align: center;
	}

	.comparison-card__header-title {
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.comparison-card__body {
		padding: 1.5rem;
	}

	.comparison-card__row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 0;
	}

	.comparison-card__row + .comparison-card__row {
		border-top: 1px solid var(--color-grey-100);
	}

	.comparison-card__label {
		color: var(--color-grey-700);
	}
	.comparison-card__label--green {
		color: var(--color-green);
		font-weight: var(--w-semibold);
	}

	.comparison-card__value {
		color: var(--color-grey-800);
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
	}
	.comparison-card__value--teal {
		color: var(--color-teal);
	}
	.comparison-card__value--green-lg {
		color: var(--color-green);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
	}

	/* CTA section */
	.page-cta__desc {
		color: var(--color-grey-700);
		line-height: 1.65;
		margin-bottom: 2.5rem;
	}

	.page-cta__actions {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.page-cta__actions {
			flex-direction: row;
			justify-content: center;
		}
	}

	.page-cta-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border-radius: var(--radius-xl);
		padding: 1rem 2rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		background-color: var(--color-teal);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
		transition: all 300ms var(--ease-out);
		cursor: pointer;
		border: none;
	}

	.page-cta-btn:hover {
		background-color: var(--color-teal-light);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-xl),
			0 8px 20px rgba(15, 164, 175, 0.3);
	}

	.page-cta-btn:active {
		transform: scale(0.97);
	}
	.page-cta-btn:disabled {
		pointer-events: none;
		opacity: 0.5;
	}

	.page-cta__link {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		transition: color 200ms var(--ease-out);
	}

	.page-cta__link:hover {
		color: var(--color-teal-light);
	}

	.page-cta__fine-print {
		color: var(--color-grey-500);
		margin-top: 2rem;
		font-size: var(--fs-xs);
	}
</style>
