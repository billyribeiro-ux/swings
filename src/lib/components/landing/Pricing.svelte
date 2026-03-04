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

	const monthlyPriceId = env.PUBLIC_STRIPE_MONTHLY_PRICE_ID || '';
	const annualPriceId = env.PUBLIC_STRIPE_ANNUAL_PRICE_ID || '';
</script>

<section id="pricing" class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Pricing"
				title="Straightforward Pricing. No Contracts. Cancel Anytime."
			/>

			<div class="mx-auto grid max-w-[820px] gap-6 sm:gap-8 md:grid-cols-2">
				{#each pricingPlans as plan, i}
					<div
						class="reveal-item group relative overflow-hidden rounded-2xl bg-white p-7 transition-all duration-500 ease-out sm:p-8 {plan.featured
							? 'ring-teal shadow-teal/8 hover:shadow-teal/12 shadow-lg ring-2 hover:shadow-xl'
							: 'ring-grey-200 hover:ring-teal/30 ring-1 hover:shadow-lg'}"
						style="transition-delay: {i * 0.08}s"
					>
						<!-- Badge -->
						{#if plan.badge}
							<div class="absolute -top-px left-1/2 -translate-x-1/2">
								<span
									class="bg-teal inline-block rounded-b-lg px-5 py-1.5 text-[11px] font-semibold tracking-wide text-white uppercase"
								>
									{plan.badge}
								</span>
							</div>
						{/if}

						<!-- Plan Name -->
						<h3
							class="text-grey-500 font-ui mb-4 text-xs font-semibold tracking-widest uppercase {plan.badge
								? 'mt-4'
								: ''}"
						>
							{plan.name}
						</h3>

						<!-- Price -->
						<div class="mb-2 flex items-baseline gap-1">
							<span class="text-navy font-heading text-4xl font-bold sm:text-5xl"
								>${plan.amount}</span
							>
							<span class="text-grey-400 text-sm">/{plan.suffix}</span>
						</div>

						<!-- Note -->
						<p class="text-grey-500 mb-6 text-sm">{plan.note}</p>

						<!-- Savings Badge -->
						{#if plan.savings}
							<div class="mb-6">
								<span class="bg-green/10 text-green rounded-lg px-3 py-1 text-xs font-semibold">
									{plan.savings}
								</span>
							</div>
						{/if}

						<!-- CTA -->
						<button
							onclick={() =>
								handleCheckout(plan.id === 'monthly' ? monthlyPriceId : annualPriceId, plan.id)}
							disabled={isLoading === plan.id}
							class="w-full rounded-xl px-6 py-3.5 text-sm font-semibold transition-all duration-300 ease-out active:scale-[0.97] {plan.variant ===
							'primary'
								? 'bg-teal shadow-teal/25 hover:bg-teal-light hover:shadow-teal/30 text-white shadow-lg hover:-translate-y-px hover:shadow-xl'
								: 'border-navy/80 text-navy hover:bg-navy hover:shadow-navy/15 border-2 bg-transparent hover:-translate-y-px hover:text-white hover:shadow-lg'} disabled:pointer-events-none disabled:opacity-50"
						>
							{isLoading === plan.id ? 'Loading...' : plan.cta}
						</button>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>
