<script lang="ts">
	import { onMount } from 'svelte';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import { pricingPlans, PRICING_ANNUAL_SAVINGS_PCT_LABEL } from '$lib/data/pricing';
	import { createCheckoutSession } from '$lib/utils/checkout';
	import { hoverTilt } from '$lib/utils/animations';
	import { ctaImpression, trackCtaEvent } from '$lib/analytics/cta';
	import { getActivePricingPlans } from '$lib/api/publicPricing';
	import CheckIcon from 'phosphor-svelte/lib/CheckIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import LockSimpleIcon from 'phosphor-svelte/lib/LockSimpleIcon';

	let isLoading = $state<string | null>(null);
	let hoveredCard = $state<string | null>(null);
	let plansForDisplay = $state(pricingPlans);

	function ctaIdForPlan(planId: string): string {
		return `pricing_${planId}`;
	}

	async function handleCheckout(planId: string): Promise<void> {
		trackCtaEvent('click', ctaIdForPlan(planId));

		isLoading = planId;
		try {
			await createCheckoutSession(planId);
		} catch (error) {
			console.error('Checkout failed:', error);
			alert('Failed to start checkout. Please try again.');
			isLoading = null;
		}
	}

	onMount(() => {
		void (async () => {
			try {
				const livePlans = await getActivePricingPlans();
				const bySlug = new Map(livePlans.map((plan) => [plan.slug, plan]));
				const monthly = bySlug.get('monthly');
				const annual = bySlug.get('annual');

				let annualSavingsLabel = PRICING_ANNUAL_SAVINGS_PCT_LABEL;
				if (monthly && annual && monthly.amount_cents > 0) {
					const yearlyMonthlyCents = monthly.amount_cents * 12;
					const savingsPct = Math.round(
						((yearlyMonthlyCents - annual.amount_cents) / yearlyMonthlyCents) * 100
					);
					annualSavingsLabel = `Save ${savingsPct}%`;
				}

				plansForDisplay = pricingPlans.map((plan) => {
					const live = bySlug.get(plan.id);
					if (!live) return plan;
					const amount = Math.round(live.amount_cents / 100);
					return {
						...plan,
						amount,
						savings: plan.id === 'annual' ? annualSavingsLabel : plan.savings
					};
				});
			} catch {
				plansForDisplay = pricingPlans;
			}
		})();
	});
</script>

<section id="pricing" class="pricing" aria-labelledby="pricing-heading">
	<div class="pricing__container">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Pricing"
				title="Straightforward Pricing. No Contracts. Cancel Anytime."
			/>

			<div class="pricing__grid">
				{#each plansForDisplay as plan, i (plan.id)}
					{@const isFeatured = plan.featured}
					<div
						class={[
							'reveal-item',
							'pricing__card',
							{
								'pricing__card--featured': isFeatured,
								'pricing__card--hovered': hoveredCard === plan.id
							}
						]}
						style="--card-delay: {i * 0.1}s"
						role="article"
						aria-label="{plan.name} plan"
						onmouseenter={() => (hoveredCard = plan.id)}
						onmouseleave={() => (hoveredCard = null)}
						{@attach ctaImpression({ ctaId: ctaIdForPlan(plan.id) })}
						{@attach hoverTilt({
							maxTilt: isFeatured ? 8 : 4,
							scale: isFeatured ? 1.03 : 1.01
						})}
					>
						<!-- Ambient light effect for featured -->
						{#if isFeatured}
							<div class="pricing__card-glow" aria-hidden="true"></div>
						{/if}

						<!-- Top edge accent -->
						<div class="pricing__card-edge" aria-hidden="true"></div>

						<!-- Badge -->
						{#if plan.badge}
							<div class="pricing__badge-wrap">
								<span class="pricing__badge">{plan.badge}</span>
							</div>
						{/if}

						<!-- Plan header -->
						<div class="pricing__header">
							<h3
								class={[
									'pricing__plan-name',
									{ 'pricing__plan-name--spaced': plan.badge }
								]}
							>
								{plan.name}
							</h3>
						</div>

						<!-- Price display -->
						<div class="pricing__price-block">
							<span class="pricing__currency" aria-hidden="true">$</span>
							<span class="pricing__amount">{plan.amount}</span>
							<span class="pricing__suffix">/{plan.suffix}</span>
						</div>

						<!-- Note -->
						<p class="pricing__note">{plan.note}</p>

						<!-- Savings indicator -->
						{#if plan.savings}
							<div class="pricing__savings-wrap">
								<span class="pricing__savings">
									<span class="pricing__savings-icon" aria-hidden="true">
										<CheckIcon size={14} weight="bold" />
									</span>
									{plan.savings}
								</span>
							</div>
						{/if}

						<!-- CTA button -->
						<button
							type="button"
							onclick={() => handleCheckout(plan.id)}
							disabled={isLoading === plan.id}
							class={[
								'pricing__cta',
								{
									'pricing__cta--primary': plan.variant === 'primary',
									'pricing__cta--outline': plan.variant !== 'primary'
								}
							]}
							aria-busy={isLoading === plan.id}
						>
							{#if isLoading === plan.id}
								<span class="pricing__spinner" aria-hidden="true"></span>
								<span>Processing…</span>
							{:else}
								<span>{plan.cta}</span>
								<span class="pricing__cta-arrow" aria-hidden="true">
									<ArrowRightIcon size={18} weight="bold" />
								</span>
							{/if}
						</button>

						<!-- Trust line -->
						<p class="pricing__trust">
							<span class="pricing__trust-icon" aria-hidden="true">
								<LockSimpleIcon size={14} weight="fill" />
							</span>
							Secure checkout via Stripe
						</p>
					</div>
				{/each}
			</div>

			<!-- Bottom guarantee -->
			<p class="pricing__guarantee">
				Not sure yet? Every plan includes full access from day one. No hidden fees, no
				upsells.
			</p>
		</ScrollReveal>
	</div>
</section>

<style>
	/* ═══════════════════════════════════════════════════════════════════════
	   PRICING SECTION — PE7 Architecture
	   ═══════════════════════════════════════════════════════════════════════
	   @layer cascade: reset → tokens → base → layout → components → animations
	   OKLCH color space · Logical properties · Native CSS nesting
	   Fluid clamp() typography · Container queries
	   Mobile-first breakpoints: 320 → 640 → 768 → 1024 → 1280 → 1536 → 1920 → 2560 → 3840
	   ═══════════════════════════════════════════════════════════════════════ */

	/* ── Section tokens (scoped) ─────────────────────────────────────────── */
	.pricing {
		--_section-pad-block: clamp(3rem, 6vw, 7rem);
		--_section-pad-inline: clamp(1rem, 3vw, 2rem);
		--_grid-gap: clamp(1.25rem, 2vw, 2rem);
		--_card-pad: clamp(1.5rem, 2.5vw, 2.25rem);
		--_card-radius: var(--radius-2xl, 1.25rem);
		--_card-border: oklch(0.76 0.035 220);
		--_card-bg: oklch(0.985 0.01 220);

		/* Brand OKLCH */
		--_teal: oklch(0.68 0.14 192);
		--_teal-light: oklch(0.74 0.14 192);
		--_teal-glow: oklch(0.68 0.14 192 / 0.12);
		--_teal-glow-strong: oklch(0.68 0.14 192 / 0.2);
		--_navy: oklch(0.25 0.05 255);
		--_navy-mid: oklch(0.33 0.055 250);
		--_navy-deep: oklch(0.22 0.055 255);
		--_green: oklch(0.74 0.14 192);
		--_green-bg: oklch(0.68 0.14 192 / 0.12);

		/* Text OKLCH */
		--_text-primary: oklch(0.22 0.04 260);
		--_text-secondary: oklch(0.45 0.02 260);
		--_text-muted: oklch(0.55 0.015 260);
		--_text-on-teal: oklch(1 0 0);
		--_text-on-navy: oklch(1 0 0);

		/* Surfaces */
		--_surface-section: oklch(0.985 0.003 250);

		/* Shadows — layered for depth */
		--_shadow-card:
			0 1px 2px oklch(0.22 0.04 260 / 0.05), 0 18px 44px -34px oklch(0.22 0.04 260 / 0.45);
		--_shadow-card-hover:
			0 12px 28px -18px oklch(0.22 0.04 260 / 0.28),
			0 24px 60px -38px oklch(0.22 0.04 260 / 0.5);
		--_shadow-featured:
			0 18px 44px -24px oklch(0.68 0.14 192 / 0.38),
			0 28px 80px -48px oklch(0.18 0.045 260 / 0.8);
		--_shadow-featured-hover:
			0 20px 52px -24px oklch(0.68 0.14 192 / 0.52),
			0 34px 90px -52px oklch(0.18 0.045 260 / 0.85);

		/* Timing */
		--_dur-fast: 200ms;
		--_dur-normal: 350ms;
		--_dur-slow: 500ms;
		--_ease-out: cubic-bezier(0.16, 1, 0.3, 1);
		--_ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);

		background-color: var(--_surface-section);
		padding-block: var(--_section-pad-block);
		position: relative;
		overflow: hidden;
	}

	/* ── Subtle background texture ───────────────────────────────────────── */
	.pricing::before {
		content: '';
		position: absolute;
		inset: 0;
		background:
			radial-gradient(
				ellipse 80% 60% at 50% 0%,
				oklch(0.68 0.14 192 / 0.03),
				transparent 70%
			),
			radial-gradient(
				ellipse 60% 50% at 80% 100%,
				oklch(0.68 0.14 192 / 0.02),
				transparent 60%
			);
		pointer-events: none;
	}

	/* ── Container ───────────────────────────────────────────────────────── */
	.pricing__container {
		position: relative;
		max-inline-size: var(--container-max, 80rem);
		margin-inline: auto;
		padding-inline: var(--_section-pad-inline);
	}

	/* ── Grid ────────────────────────────────────────────────────────────── */
	.pricing__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: var(--_grid-gap);
		max-inline-size: 64rem;
		margin-inline: auto;
		margin-block-start: clamp(2rem, 4vw, 3rem);
	}

	/* ── Card ────────────────────────────────────────────────────────────── */
	.pricing__card {
		position: relative;
		container-type: inline-size;
		container-name: pricing-card;
		display: flex;
		min-block-size: clamp(20rem, 34vw, 23.5rem);
		flex-direction: column;
		overflow: hidden;
		border-radius: var(--_card-radius);
		background:
			radial-gradient(
				ellipse 80% 55% at 20% 0%,
				oklch(0.68 0.14 192 / 0.11),
				transparent 64%
			),
			radial-gradient(
				ellipse 60% 50% at 100% 100%,
				oklch(0.74 0.14 192 / 0.07),
				transparent 66%
			),
			linear-gradient(180deg, oklch(1 0 0) 0%, var(--_card-bg) 100%);
		padding: var(--_card-pad);
		padding-block-start: calc(var(--_card-pad) + 0.25rem);
		outline: 1px solid var(--_card-border);
		outline-offset: -1px;
		box-shadow:
			var(--_shadow-card),
			inset 0 1px 0 rgba(255, 255, 255, 1);
		transition:
			box-shadow var(--_dur-normal) var(--_ease-out),
			outline-color var(--_dur-normal) var(--_ease-out),
			transform var(--_dur-normal) var(--_ease-out);
		transition-delay: var(--card-delay, 0s);
	}

	.pricing__card:hover {
		box-shadow:
			var(--_shadow-card-hover),
			inset 0 1px 0 rgba(255, 255, 255, 1);
		outline-color: oklch(0.68 0.14 192 / 0.3);
		transform: translateY(-2px);
	}

	.pricing__card:not(.pricing__card--featured)::before {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		background:
			linear-gradient(180deg, rgba(255, 255, 255, 0.8), transparent 45%),
			linear-gradient(135deg, oklch(0.68 0.14 192 / 0.1), transparent 42%);
		pointer-events: none;
	}

	/* Featured card */
	.pricing__card--featured {
		outline: 1px solid oklch(0.68 0.14 192 / 0.45);
		background:
			radial-gradient(
				ellipse 90% 70% at 50% -10%,
				oklch(0.68 0.14 192 / 0.3),
				transparent 66%
			),
			radial-gradient(
				ellipse 70% 55% at 100% 100%,
				oklch(0.74 0.14 192 / 0.16),
				transparent 70%
			),
			linear-gradient(
				145deg,
				var(--_navy) 0%,
				var(--_navy-mid) 58%,
				oklch(0.29 0.06 245) 100%
			);
		box-shadow:
			var(--_shadow-featured),
			inset 0 1px 1px rgba(255, 255, 255, 0.12);
	}

	.pricing__card--featured::before {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		padding: 2px;
		background: conic-gradient(
			from var(--border-angle) at 50% 50%,
			transparent,
			var(--_teal),
			var(--_teal-light),
			transparent 25%
		);
		-webkit-mask:
			linear-gradient(#fff 0 0) content-box,
			linear-gradient(#fff 0 0);
		mask:
			linear-gradient(#fff 0 0) content-box,
			linear-gradient(#fff 0 0);
		-webkit-mask-composite: xor;
		mask-composite: exclude;
		pointer-events: none;
		animation: spin 4s linear infinite;
	}

	@property --border-angle {
		syntax: '<angle>';
		inherits: true;
		initial-value: 0turn;
	}

	@keyframes spin {
		to {
			--border-angle: 1turn;
		}
	}

	.pricing__card--featured:hover {
		box-shadow:
			var(--_shadow-featured-hover),
			inset 0 1px 1px rgba(255, 255, 255, 0.1);
		transform: translateY(-4px);
	}

	/* Text overrides for dark featured card */
	.pricing__card--featured .pricing__plan-name {
		color: rgba(255, 255, 255, 0.86);
	}

	.pricing__card--featured .pricing__note,
	.pricing__card--featured .pricing__suffix,
	.pricing__card--featured .pricing__trust {
		color: rgba(255, 255, 255, 0.76);
	}

	/* ── Card glow (featured only) ───────────────────────────────────────── */
	.pricing__card-glow {
		position: absolute;
		inset-block-start: -50%;
		inset-inline-start: -25%;
		inline-size: 150%;
		block-size: 150%;
		background: radial-gradient(ellipse 50% 40% at 50% 20%, var(--_teal-glow), transparent 70%);
		pointer-events: none;
		opacity: 0.6;
		transition: opacity var(--_dur-slow) var(--_ease-out);
	}

	.pricing__card--featured:hover .pricing__card-glow {
		opacity: 1;
	}

	/* ── Top edge accent ─────────────────────────────────────────────────── */
	.pricing__card-edge {
		position: absolute;
		inset-block-start: 0;
		inset-inline: 0;
		block-size: 3px;
		background: linear-gradient(
			90deg,
			transparent 0%,
			var(--_teal) 20%,
			var(--_teal-light) 50%,
			var(--_teal) 80%,
			transparent 100%
		);
		transition: background var(--_dur-normal) var(--_ease-out);
	}

	.pricing__card--featured .pricing__card-edge {
		block-size: 3px;
		background: linear-gradient(
			90deg,
			transparent 0%,
			var(--_teal) 15%,
			var(--_teal-light) 50%,
			var(--_teal) 85%,
			transparent 100%
		);
	}

	.pricing__card:hover .pricing__card-edge {
		background: linear-gradient(
			90deg,
			transparent 0%,
			var(--_teal) 20%,
			var(--_teal) 80%,
			transparent 100%
		);
	}

	/* ── Badge ───────────────────────────────────────────────────────────── */
	.pricing__badge-wrap {
		position: absolute;
		inset-block-start: 0;
		inset-inline-start: 50%;
		transform: translateX(-50%);
		z-index: 2;
	}

	.pricing__badge {
		display: inline-block;
		background: linear-gradient(135deg, var(--_teal), var(--_teal-light));
		border-radius: 0 0 var(--radius-lg, 0.75rem) var(--radius-lg, 0.75rem);
		padding-block: 0.45rem;
		padding-inline: 1.35rem;
		font-size: clamp(0.625rem, 1.2cqi, 0.6875rem);
		font-weight: var(--w-semibold, 600);
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--_text-on-teal);
		box-shadow:
			0 10px 24px -18px oklch(0.68 0.14 192 / 0.9),
			inset 0 1px 0 rgba(255, 255, 255, 0.22);
	}

	/* ── Plan header ─────────────────────────────────────────────────────── */
	.pricing__header {
		position: relative;
		z-index: 1;
	}

	.pricing__plan-name {
		color: oklch(0.29 0.06 250);
		font-family: var(--font-ui, system-ui);
		font-size: clamp(0.6875rem, 1.5cqi, 0.75rem);
		font-weight: var(--w-semibold, 600);
		letter-spacing: 0.1em;
		text-transform: uppercase;
		margin-block-end: 0.875rem;
	}

	.pricing__plan-name--spaced {
		margin-block-start: 1.125rem;
	}

	/* ── Price block ─────────────────────────────────────────────────────── */
	.pricing__price-block {
		position: relative;
		z-index: 1;
		display: flex;
		align-items: baseline;
		gap: 0.125rem;
		margin-block-end: 0.5rem;
	}

	.pricing__currency {
		color: var(--_text-muted);
		font-family: var(--font-heading, system-ui);
		font-size: clamp(1.25rem, 3cqi, 1.5rem);
		font-weight: var(--w-bold, 700);
		align-self: flex-start;
		margin-block-start: 0.5rem;
	}

	.pricing__amount {
		color: var(--_text-primary);
		font-family: var(--font-heading, system-ui);
		font-size: clamp(2.75rem, 8cqi, 3.5rem);
		font-weight: var(--w-bold, 700);
		line-height: 1;
		letter-spacing: -0.03em;
	}

	/* After base `.pricing__amount` so featured legibility always wins */
	.pricing__card--featured .pricing__amount {
		color: #fff;
		background: none;
		background-clip: border-box;
		-webkit-background-clip: border-box;
		-webkit-text-fill-color: #fff;
	}

	.pricing__card--featured .pricing__currency {
		color: rgba(255, 255, 255, 0.9);
	}

	.pricing__suffix {
		color: var(--_text-muted);
		font-size: clamp(0.875rem, 2cqi, 1rem);
		font-weight: var(--w-medium, 500);
		margin-inline-start: 0.125rem;
	}

	/* ── Note ────────────────────────────────────────────────────────────── */
	.pricing__note {
		position: relative;
		z-index: 1;
		color: var(--_text-secondary);
		font-size: clamp(0.8125rem, 1.8cqi, 0.9375rem);
		line-height: 1.5;
		margin-block-end: 1.1rem;
	}

	/* ── Savings ─────────────────────────────────────────────────────────── */
	.pricing__savings-wrap {
		position: relative;
		z-index: 1;
		margin-block-end: 1.25rem;
	}

	.pricing__savings {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		background-color: var(--_green-bg);
		color: var(--_green);
		border: 1px solid oklch(0.68 0.14 192 / 0.18);
		border-radius: var(--radius-lg, 0.75rem);
		padding-block: 0.3125rem;
		padding-inline: 0.75rem;
		font-size: clamp(0.6875rem, 1.5cqi, 0.8125rem);
		font-weight: var(--w-semibold, 600);
	}

	.pricing__savings-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		inline-size: 0.875rem;
		block-size: 0.875rem;
		flex-shrink: 0;
	}

	.pricing__savings-icon :global(svg) {
		inline-size: 100%;
		block-size: 100%;
	}

	/* ── CTA Button ──────────────────────────────────────────────────────── */
	.pricing__cta {
		position: relative;
		z-index: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		inline-size: 100%;
		border-radius: var(--radius-xl, 1rem);
		margin-block-start: auto;
		padding-block: 0.9375rem;
		padding-inline: 1.5rem;
		font-size: clamp(0.875rem, 2cqi, 0.9375rem);
		font-weight: var(--w-semibold, 600);
		letter-spacing: 0.01em;
		cursor: pointer;
		border: none;
		transition:
			background-color var(--_dur-fast) var(--_ease-out),
			color var(--_dur-fast) var(--_ease-out),
			box-shadow var(--_dur-normal) var(--_ease-out),
			transform var(--_dur-fast) var(--_ease-spring);
	}

	.pricing__cta:active {
		transform: scale(0.97);
	}

	.pricing__cta:disabled {
		pointer-events: none;
		opacity: 0.55;
	}

	/* Primary CTA */
	.pricing__cta--primary {
		background: linear-gradient(135deg, var(--_teal), var(--_teal-light));
		color: var(--_text-on-teal);
		box-shadow:
			0 2px 8px oklch(0.68 0.14 192 / 0.2),
			0 6px 20px oklch(0.68 0.14 192 / 0.15);
	}

	.pricing__cta--primary:hover {
		background: linear-gradient(135deg, var(--_teal-light), var(--_teal));
		transform: translateY(-1px);
		box-shadow:
			0 4px 12px oklch(0.68 0.14 192 / 0.25),
			0 12px 32px oklch(0.68 0.14 192 / 0.18);
	}

	/* Outline CTA */
	.pricing__cta--outline {
		background:
			linear-gradient(180deg, rgba(255, 255, 255, 0.92), rgba(255, 255, 255, 0.72)),
			transparent;
		color: oklch(0.25 0.05 255);
		border: 1px solid oklch(0.68 0.14 192 / 0.45);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.95),
			0 10px 24px -22px oklch(0.68 0.14 192 / 0.65);
	}

	.pricing__cta--outline:hover {
		background: linear-gradient(135deg, var(--_navy), var(--_navy-mid));
		color: var(--_text-on-navy);
		border-color: oklch(0.68 0.14 192 / 0.5);
		transform: translateY(-1px);
		box-shadow:
			0 4px 12px oklch(0.22 0.04 260 / 0.16),
			0 12px 30px oklch(0.68 0.14 192 / 0.16);
	}

	/* CTA arrow */
	.pricing__cta-arrow {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		inline-size: 1.125rem;
		block-size: 1.125rem;
		flex-shrink: 0;
		transition: transform var(--_dur-fast) var(--_ease-out);
	}

	.pricing__cta-arrow :global(svg) {
		inline-size: 100%;
		block-size: 100%;
	}

	.pricing__cta:hover .pricing__cta-arrow {
		transform: translateX(3px);
	}

	/* ── Spinner ─────────────────────────────────────────────────────────── */
	.pricing__spinner {
		inline-size: 1.125rem;
		block-size: 1.125rem;
		border: 2px solid oklch(1 0 0 / 0.3);
		border-block-start-color: oklch(1 0 0);
		border-radius: 50%;
		animation: pricing-spin 0.6s linear infinite;
	}

	@keyframes pricing-spin {
		to {
			transform: rotate(360deg);
		}
	}

	/* ── Trust line ──────────────────────────────────────────────────────── */
	.pricing__trust {
		position: relative;
		z-index: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.375rem;
		margin-block-start: 0.9rem;
		color: var(--_text-muted);
		font-size: clamp(0.6875rem, 1.4cqi, 0.75rem);
	}

	.pricing__trust-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		inline-size: 0.875rem;
		block-size: 0.875rem;
		flex-shrink: 0;
		opacity: 0.6;
	}

	.pricing__trust-icon :global(svg) {
		inline-size: 100%;
		block-size: 100%;
	}

	/* ── Guarantee ───────────────────────────────────────────────────────── */
	.pricing__guarantee {
		text-align: center;
		color: var(--_text-muted);
		font-size: clamp(0.8125rem, 1.5vw, 0.9375rem);
		line-height: 1.6;
		max-inline-size: 48ch;
		margin-inline: auto;
		margin-block-start: clamp(1.5rem, 3vw, 2.5rem);
	}

	/* ═══════════════════════════════════════════════════════════════════════
	   RESPONSIVE BREAKPOINTS — Mobile-first, 9-tier PE7 system
	   0 → 640 → 768 → 1024 → 1280 → 1536 → 1920 → 2560 → 3840
	   ═══════════════════════════════════════════════════════════════════════ */

	/* ── sm: 640px — Large phones ─────────────────────────────────────── */
	@media (width >= 640px) {
		.pricing__card {
			padding: calc(var(--_card-pad) + 0.25rem);
		}
	}

	/* ── md: 768px — Tablet portrait — 2-col grid ────────────────────── */
	@media (width >= 768px) {
		.pricing__grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		.pricing__card--featured {
			z-index: 1;
		}

		.pricing__card--featured:hover {
			transform: translateY(-4px);
		}
	}

	/* ── lg: 1024px — Tablet landscape / small desktop ───────────────── */
	@media (width >= 1024px) {
		.pricing {
			--_section-pad-block: clamp(5rem, 8vw, 8rem);
		}

		.pricing__grid {
			gap: 2.25rem;
		}

		.pricing__card {
			padding: 2.25rem;
			border-radius: 1.5rem;
		}
	}

	/* ── xl: 1280px — Desktop ────────────────────────────────────────── */
	@media (width >= 1280px) {
		.pricing__container {
			padding-inline: 2rem;
		}

		.pricing__grid {
			max-inline-size: 60rem;
			gap: 2.5rem;
		}

		.pricing__card {
			padding: 2.5rem;
		}

		.pricing__amount {
			font-size: 3.75rem;
		}
	}

	/* ── 2xl: 1536px — Wide desktop ──────────────────────────────────── */
	@media (width >= 1536px) {
		.pricing__container {
			padding-inline: 3rem;
		}

		.pricing__grid {
			max-inline-size: 64rem;
			gap: 3rem;
		}

		.pricing__card {
			padding: 2.75rem;
		}
	}

	/* ── 3xl: 1920px — Full HD ───────────────────────────────────────── */
	@media (width >= 1920px) {
		.pricing {
			--_section-pad-block: 9rem;
		}

		.pricing__container {
			max-inline-size: 90rem;
		}

		.pricing__grid {
			max-inline-size: 68rem;
		}

		.pricing__card {
			padding: 3rem;
			border-radius: 1.75rem;
		}

		.pricing__amount {
			font-size: 4rem;
		}
	}

	/* ── 4K: 2560px — QHD / Retina displays ──────────────────────────── */
	@media (width >= 2560px) {
		.pricing {
			--_section-pad-block: 11rem;
		}

		.pricing__container {
			max-inline-size: 110rem;
		}

		.pricing__grid {
			max-inline-size: 76rem;
			gap: 3.5rem;
		}

		.pricing__card {
			padding: 3.5rem;
			border-radius: 2rem;
		}

		.pricing__badge {
			padding-block: 0.5rem;
			padding-inline: 1.5rem;
			font-size: 0.8125rem;
		}

		.pricing__plan-name {
			font-size: 0.875rem;
		}

		.pricing__amount {
			font-size: 4.5rem;
		}

		.pricing__suffix {
			font-size: 1.125rem;
		}

		.pricing__note {
			font-size: 1.0625rem;
		}

		.pricing__cta {
			padding-block: 1.125rem;
			padding-inline: 2rem;
			font-size: 1.0625rem;
			border-radius: 1.25rem;
		}

		.pricing__guarantee {
			font-size: 1.0625rem;
		}
	}

	/* ── 5K: 3840px — Ultra-wide / Studio displays ───────────────────── */
	@media (width >= 3840px) {
		.pricing {
			--_section-pad-block: 14rem;
		}

		.pricing__container {
			max-inline-size: 140rem;
		}

		.pricing__grid {
			max-inline-size: 90rem;
			gap: 4rem;
		}

		.pricing__card {
			padding: 4rem;
			border-radius: 2.5rem;
		}

		.pricing__card-edge {
			block-size: 4px;
		}

		.pricing__badge {
			padding-block: 0.625rem;
			padding-inline: 2rem;
			font-size: 0.9375rem;
			border-radius: 0 0 1rem 1rem;
		}

		.pricing__plan-name {
			font-size: 1rem;
			letter-spacing: 0.12em;
			margin-block-end: 1.25rem;
		}

		.pricing__plan-name--spaced {
			margin-block-start: 1.5rem;
		}

		.pricing__currency {
			font-size: 2rem;
			margin-block-start: 0.75rem;
		}

		.pricing__amount {
			font-size: 5.5rem;
		}

		.pricing__suffix {
			font-size: 1.375rem;
		}

		.pricing__note {
			font-size: 1.1875rem;
			margin-block-end: 1.75rem;
		}

		.pricing__savings {
			font-size: 1rem;
			padding-block: 0.4375rem;
			padding-inline: 1rem;
		}

		.pricing__savings-icon {
			inline-size: 1.125rem;
			block-size: 1.125rem;
		}

		.pricing__cta {
			padding-block: 1.375rem;
			padding-inline: 2.5rem;
			font-size: 1.1875rem;
			border-radius: 1.5rem;
		}

		.pricing__cta-arrow {
			inline-size: 1.375rem;
			block-size: 1.375rem;
		}

		.pricing__trust {
			font-size: 0.9375rem;
			margin-block-start: 1.125rem;
			gap: 0.5rem;
		}

		.pricing__trust-icon {
			inline-size: 1.125rem;
			block-size: 1.125rem;
		}

		.pricing__guarantee {
			font-size: 1.1875rem;
			margin-block-start: 3.5rem;
		}
	}

	/* ═══════════════════════════════════════════════════════════════════════
	   CONTAINER QUERIES — Component-level responsiveness
	   ═══════════════════════════════════════════════════════════════════════ */

	@container pricing-card (inline-size >= 22rem) {
		.pricing__price-block {
			gap: 0.25rem;
		}

		.pricing__cta {
			padding-block: 1rem;
		}
	}

	@container pricing-card (inline-size >= 28rem) {
		.pricing__header {
			display: flex;
			align-items: center;
			justify-content: space-between;
		}
	}

	/* ═══════════════════════════════════════════════════════════════════════
	   ACCESSIBILITY — Reduced motion
	   ═══════════════════════════════════════════════════════════════════════ */

	@media (prefers-reduced-motion: reduce) {
		.pricing__card,
		.pricing__cta,
		.pricing__cta-arrow,
		.pricing__card-glow,
		.pricing__card-edge {
			transition-duration: 0.01ms !important;
		}

		.pricing__spinner {
			animation-duration: 1.5s;
		}
	}

	/* ═══════════════════════════════════════════════════════════════════════
	   HIGH CONTRAST — Forced colors mode
	   ═══════════════════════════════════════════════════════════════════════ */

	@media (forced-colors: active) {
		.pricing__card {
			border: 2px solid CanvasText;
		}

		.pricing__card--featured {
			border: 3px solid Highlight;
		}

		.pricing__cta--primary {
			background: Highlight;
			color: HighlightText;
		}

		.pricing__cta--outline {
			border: 2px solid ButtonText;
		}

		.pricing__card-glow,
		.pricing__card-edge,
		.pricing::before {
			display: none;
		}
	}
</style>
