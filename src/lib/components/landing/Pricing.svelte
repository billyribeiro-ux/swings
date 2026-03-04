<script lang="ts">
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { pricingPlans } from '$lib/data/pricing';
	import { createCheckoutSession } from '$lib/utils/stripe';
	import { env } from '$env/dynamic/public';

	let isLoading = $state<string | null>(null);

	async function handleCheckout(priceId: string, planId: string) {
		if (!priceId) {
			alert('Stripe is not configured. Please add your Stripe Price IDs to the .env file.');
			return;
		}

		isLoading = planId;
		try {
			await createCheckoutSession(priceId);
		} catch (error) {
			console.error('Checkout failed:', error);
			alert('Failed to start checkout. Please try again.');
			isLoading = null;
		}
	}

	const monthlyPriceId = env.STRIPE_MONTHLY_PRICE_ID || '';
	const annualPriceId = env.STRIPE_ANNUAL_PRICE_ID || '';
</script>

<section id="pricing" class="bg-white py-20 lg:py-32">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Pricing"
				title="Straightforward Pricing. No Contracts. Cancel Anytime."
			/>

			<div class="mx-auto grid max-w-3xl gap-6 md:grid-cols-2">
				{#each pricingPlans as plan, i}
					<div
						class="reveal-item relative rounded-xl bg-white p-8 {plan.featured
							? 'border-teal border-2'
							: 'border-grey-200 border-2'}"
						style="transition-delay: {i * 0.15}s"
					>
						<!-- Badge -->
						{#if plan.badge}
							<div class="absolute -top-3 left-1/2 -translate-x-1/2">
								<span
									class="bg-teal rounded-full px-4 py-1 text-xs font-semibold tracking-wide text-white uppercase"
								>
									{plan.badge}
								</span>
							</div>
						{/if}

						<!-- Plan Name -->
						<h3 class="text-grey-600 font-ui mb-3 text-sm font-semibold tracking-wide uppercase">
							{plan.name}
						</h3>

						<!-- Price -->
						<div class="mb-3 flex items-baseline gap-1">
							<span class="text-navy font-heading text-5xl font-bold">${plan.amount}</span>
							<span class="text-grey-500 text-base">/{plan.suffix}</span>
						</div>

						<!-- Note -->
						<p class="text-grey-500 mb-6 text-sm">{plan.note}</p>

						<!-- Savings Badge -->
						{#if plan.savings}
							<div class="mb-6">
								<span class="bg-green/10 text-green rounded-md px-3 py-1 text-sm font-medium">
									{plan.savings}
								</span>
							</div>
						{/if}

						<!-- CTA -->
						<button
							onclick={() =>
								handleCheckout(plan.id === 'monthly' ? monthlyPriceId : annualPriceId, plan.id)}
							disabled={isLoading === plan.id}
							class="w-full rounded-lg px-6 py-3 text-sm font-semibold transition-all {plan.variant ===
							'primary'
								? 'bg-teal hover:bg-teal-light text-white'
								: 'border-navy text-navy hover:bg-navy border-2 bg-transparent hover:text-white'} disabled:cursor-not-allowed disabled:opacity-50"
						>
							{isLoading === plan.id ? 'Loading...' : plan.cta}
						</button>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>
