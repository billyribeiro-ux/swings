<script lang="ts">
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { pricingPlans } from '$lib/data/pricing';
	import { createCheckoutSession } from '$lib/utils/stripe';
	import { env } from '$env/dynamic/public';
	import { ctaImpression, trackCtaEvent } from '$lib/analytics/cta';

	let isLoading = $state<string | null>(null);

	function ctaIdForPlan(planId: string): string {
		return `pricing_${planId}`;
	}

	async function handleCheckout(priceId: string, planId: string) {
		if (!priceId) {
			alert('Stripe is not configured. Please add your Stripe Price IDs to the .env file.');
			return;
		}

		trackCtaEvent('click', ctaIdForPlan(planId));

		isLoading = planId;
		try {
			await createCheckoutSession(priceId);
		} catch (error) {
			console.error('Checkout failed:', error);
			alert('Failed to start checkout. Please try again.');
			isLoading = null;
		}
	}

	const monthlyPriceId = env.PUBLIC_STRIPE_MONTHLY_PRICE_ID || '';
	const annualPriceId = env.PUBLIC_STRIPE_ANNUAL_PRICE_ID || '';
</script>

<section id="pricing" class="pricing-section">
	<div class="pricing-section__container">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Pricing"
				title="Straightforward Pricing. No Contracts. Cancel Anytime."
			/>

			<div class="pricing-section__grid">
				{#each pricingPlans as plan, i (plan.id)}
					<div
						class={['reveal-item pricing-card', plan.featured && 'pricing-card--featured']}
						style="transition-delay: {i * 0.08}s"
						use:ctaImpression={{ ctaId: ctaIdForPlan(plan.id) }}
					>
						<!-- Badge -->
						{#if plan.badge}
							<div class="pricing-card__badge-wrap">
								<span class="pricing-card__badge">{plan.badge}</span>
							</div>
						{/if}

						<!-- Plan Name -->
						<h3 class={['pricing-card__name', plan.badge && 'pricing-card__name--spaced']}>
							{plan.name}
						</h3>

						<!-- Price -->
						<div class="pricing-card__price">
							<span class="pricing-card__amount">${plan.amount}</span>
							<span class="pricing-card__suffix">/{plan.suffix}</span>
						</div>

						<!-- Note -->
						<p class="pricing-card__note">{plan.note}</p>

						<!-- Savings Badge -->
						{#if plan.savings}
							<div class="pricing-card__savings-wrap">
								<span class="pricing-card__savings">{plan.savings}</span>
							</div>
						{/if}

						<!-- CTA -->
						<button
							onclick={() =>
								handleCheckout(plan.id === 'monthly' ? monthlyPriceId : annualPriceId, plan.id)}
							disabled={isLoading === plan.id}
							class={[
								'pricing-card__cta',
								plan.variant === 'primary'
									? 'pricing-card__cta--primary'
									: 'pricing-card__cta--outline'
							]}
						>
							{isLoading === plan.id ? 'Loading...' : plan.cta}
						</button>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	.pricing-section {
		background-color: var(--color-white);
		padding: 4rem 0;
	}

	@media (min-width: 640px) {
		.pricing-section {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.pricing-section {
			padding: 7rem 0;
		}
	}

	.pricing-section__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.pricing-section__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.pricing-section__container {
			padding: 0 2rem;
		}
	}

	.pricing-section__grid {
		max-width: 820px;
		margin: 0 auto;
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 640px) {
		.pricing-section__grid {
			gap: 2rem;
		}
	}
	@media (min-width: 768px) {
		.pricing-section__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.pricing-card {
		position: relative;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		background-color: var(--color-white);
		padding: 1.75rem;
		transition: all 500ms var(--ease-out);
		outline: 1px solid var(--color-grey-200);
		outline-offset: -1px;
	}

	@media (min-width: 640px) {
		.pricing-card {
			padding: 2rem;
		}
	}

	.pricing-card:hover {
		box-shadow: var(--shadow-lg);
		outline-color: rgba(15, 164, 175, 0.3);
	}

	.pricing-card--featured {
		outline: 2px solid var(--color-teal);
		box-shadow:
			var(--shadow-lg),
			0 4px 20px rgba(15, 164, 175, 0.08);
	}

	.pricing-card--featured:hover {
		box-shadow:
			var(--shadow-xl),
			0 8px 30px rgba(15, 164, 175, 0.12);
	}

	.pricing-card__badge-wrap {
		position: absolute;
		top: -1px;
		left: 50%;
		transform: translateX(-50%);
	}

	.pricing-card__badge {
		display: inline-block;
		background-color: var(--color-teal);
		border-radius: 0 0 var(--radius-lg) var(--radius-lg);
		padding: 0.375rem 1.25rem;
		font-size: 11px;
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--color-white);
	}

	.pricing-card__name {
		color: var(--color-grey-500);
		font-family: var(--font-ui);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin-bottom: 1rem;
	}

	.pricing-card__name--spaced {
		margin-top: 1rem;
	}

	.pricing-card__price {
		display: flex;
		align-items: baseline;
		gap: 0.25rem;
		margin-bottom: 0.5rem;
	}

	.pricing-card__amount {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-4xl);
		font-weight: var(--w-bold);
	}

	@media (min-width: 640px) {
		.pricing-card__amount {
			font-size: 3rem;
		}
	}

	.pricing-card__suffix {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.pricing-card__note {
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
	}

	.pricing-card__savings-wrap {
		margin-bottom: 1.5rem;
	}

	.pricing-card__savings {
		background-color: rgba(34, 181, 115, 0.1);
		color: var(--color-green);
		border-radius: var(--radius-lg);
		padding: 0.25rem 0.75rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
	}

	.pricing-card__cta {
		width: 100%;
		border-radius: var(--radius-xl);
		padding: 0.875rem 1.5rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		transition: all 300ms var(--ease-out);
		cursor: pointer;
	}

	.pricing-card__cta:active {
		transform: scale(0.97);
	}
	.pricing-card__cta:disabled {
		pointer-events: none;
		opacity: 0.5;
	}

	.pricing-card__cta--primary {
		background-color: var(--color-teal);
		color: var(--color-white);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
		border: none;
	}

	.pricing-card__cta--primary:hover {
		background-color: var(--color-teal-light);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-xl),
			0 8px 20px rgba(15, 164, 175, 0.3);
	}

	.pricing-card__cta--outline {
		background-color: transparent;
		color: var(--color-navy);
		border: 2px solid rgba(11, 29, 58, 0.8);
	}

	.pricing-card__cta--outline:hover {
		background-color: var(--color-navy);
		color: var(--color-white);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(11, 29, 58, 0.15);
	}
</style>
