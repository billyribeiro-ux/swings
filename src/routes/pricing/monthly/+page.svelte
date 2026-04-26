<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicCascade, EASE, DURATION } from '$lib/utils/animations';
	import { createCheckoutSession } from '$lib/utils/checkout';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { productSchema, buildJsonLd } from '$lib/seo/jsonld';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import CurrencyDollarIcon from 'phosphor-svelte/lib/CurrencyDollarIcon';
	import { PRICING_ANNUAL_SAVINGS_PERCENT_ROUNDED, PRICING_MONTHLY_USD } from '$lib/data/pricing';
	import { getActivePricingPlans } from '$lib/api/publicPricing';

	const jsonLd = buildJsonLd([
		productSchema({
			name: 'Precision Options Signals Monthly Plan',
			description:
				'Weekly options watchlists with 5-7 high-probability setups, entry zones, profit targets, and stop losses.',
			price: String(PRICING_MONTHLY_USD),
			path: '/pricing/monthly',
			billingPeriod: 'month'
		})
	]);

	let heroRef: HTMLElement | undefined = $state();
	let isLoading = $state(false);
	let monthlyPriceUsd = $state(PRICING_MONTHLY_USD);
	let annualSavingsPercent = $state(PRICING_ANNUAL_SAVINGS_PERCENT_ROUNDED);

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

		void (async () => {
			try {
				const plans = await getActivePricingPlans();
				const monthly = plans.find((plan) => plan.slug === 'monthly');
				const annual = plans.find((plan) => plan.slug === 'annual');

				if (monthly) {
					monthlyPriceUsd = Math.round(monthly.amount_cents / 100);
				}
				if (monthly && annual && monthly.amount_cents > 0) {
					const yearlyMonthlyCents = monthly.amount_cents * 12;
					annualSavingsPercent = Math.round(
						((yearlyMonthlyCents - annual.amount_cents) / yearlyMonthlyCents) * 100
					);
				}
			} catch {
				monthlyPriceUsd = PRICING_MONTHLY_USD;
				annualSavingsPercent = PRICING_ANNUAL_SAVINGS_PERCENT_ROUNDED;
			}
		})();

		return () => ctx.revert();
	});

	async function handleCheckout() {
		isLoading = true;
		try {
			await createCheckoutSession('monthly');
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
		'Cancel anytime, no long-term commitment'
	];
</script>

<Seo
	title="Monthly Plan - Precision Options Signals"
	description="Get weekly options watchlists for $49/month. Cancel anytime. 5-7 high-probability setups with entries, targets, and stops."
	ogTitle="Monthly Plan $49/mo - Precision Options Signals"
	{jsonLd}
/>

<!-- Hero -->
<section bind:this={heroRef} class="page-hero">
	<div class="page-hero__grid-overlay"></div>

	<div class="page-hero__inner">
		<div class="price-badge page-badge">
			<CurrencyDollarIcon size={18} weight="duotone" color="#15C5D1" />
			<span class="page-badge__text">Monthly Plan</span>
		</div>

		<h1 class="price-title page-hero__title">Weekly Watchlists, Month-to-Month</h1>

		<div class="price-amount price-hero__amount">
			<div class="price-hero__price-row">
				<span class="price-hero__price">{'$' + monthlyPriceUsd}</span>
				<span class="price-hero__suffix">/month</span>
			</div>
			<p class="price-hero__note">Billed monthly. Cancel anytime.</p>
		</div>
	</div>
</section>

<!-- Features -->
<section class="page-section page-section--white">
	<div class="page-container">
		<ScrollReveal>
			<div class="price-features page-narrow">
				<h2 class="page-section__heading page-section__heading--center">What's Included</h2>

				<div class="feature-list">
					{#each features as feature, i (feature)}
						<div
							class="reveal-item feature-list__item"
							style="transition-delay: {i * 0.06}s"
						>
							<CheckCircleIcon
								size={24}
								weight="fill"
								color="#0FA4AF"
								class="feature-list__icon"
							/>
							<p class="feature-list__text">{feature}</p>
						</div>
					{/each}
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- CTA -->
<section class="page-section page-section--off-white">
	<div class="page-container page-container--center">
		<ScrollReveal>
			<div class="price-cta page-narrow">
				<h2 class="reveal-item page-section__heading page-section__heading--center">
					Ready to Get Started?
				</h2>

				<p class="reveal-item page-cta__desc">
					Join hundreds of traders who trust Precision Options Signals for their weekly
					options watchlists.
				</p>

				<div class="reveal-item page-cta__actions">
					<button onclick={handleCheckout} disabled={isLoading} class="page-cta-btn">
						{#if isLoading}
							Processing...
						{:else}
							Start Monthly Plan -- {'$' + monthlyPriceUsd}/mo
							<ArrowRightIcon size={18} weight="bold" />
						{/if}
					</button>

					<a href="/pricing/annual" class="page-cta__link">
						View Annual Plan (Save {annualSavingsPercent}%)
						<ArrowRightIcon size={14} weight="bold" />
					</a>
				</div>

				<p class="reveal-item page-cta__fine-print">
					Cancel anytime. No long-term commitment required.
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
		border: 1px solid rgba(15, 164, 175, 0.3);
		background-color: rgba(15, 164, 175, 0.1);
		padding: 0.5rem 1rem;
		margin-bottom: 1.5rem;
	}

	.page-badge__text {
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
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

	.price-hero__note {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		margin-top: 0.75rem;
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
