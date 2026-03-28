<script lang="ts">
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Clock from 'phosphor-svelte/lib/Clock';
	import ShieldCheck from 'phosphor-svelte/lib/ShieldCheck';
	import CurrencyDollar from 'phosphor-svelte/lib/CurrencyDollar';

	const features = [
		{
			icon: Clock,
			title: 'Early Entry Alerts',
			description:
				"Most services notify you after the move has already happened. We send alerts before the breakout, with the exact price level we're watching - giving you time to plan, not chase."
		},
		{
			icon: ShieldCheck,
			title: 'Clear Invalidation Levels',
			description:
				"Every alert includes a simple 'no longer valid' level. If price breaks below it, the setup is done -- no confusion, no guessing, no hoping. You always know where you stand."
		},
		{
			icon: CurrencyDollar,
			title: 'Simple Profit-Taking Guidance',
			description:
				"We highlight logical profit zones so you can scale out with confidence, even if you're not an experienced trader. No complicated calculations -- just clear, actionable targets."
		}
	];
</script>

<section id="how-it-works" class="why-different">
	<div class="why-different__container">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Why We're Different"
				title="Most Alert Services Tell You <em>After</em> the Move. We Tell You <em>Before</em>."
				subtitle="Built on the proprietary 'Move Prior to The Move' methodology - your watchlist arrives Sunday night so you can set alerts and be positioned before the action starts."
			/>

			<div class="why-different__grid">
				{#each features as feature, i (feature.title)}
					<div class="reveal-item why-different__card" style="transition-delay: {i * 0.15}s">
						<div class="why-different__icon-wrap">
							<feature.icon size={24} weight="duotone" color="#0FA4AF" />
						</div>
						<h3 class="why-different__title">{feature.title}</h3>
						<p class="why-different__desc">{feature.description}</p>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	.why-different {
		padding: 5rem 0;
		background-color: var(--color-off-white);
	}

	@media (min-width: 1024px) {
		.why-different {
			padding: 8rem 0;
		}
	}

	.why-different__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.why-different__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.why-different__container {
			padding: 0 2rem;
		}
	}

	.why-different__grid {
		display: grid;
		gap: 2rem;
	}

	@media (min-width: 768px) {
		.why-different__grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.why-different__card {
		background-color: var(--color-white);
		border-radius: var(--radius-xl);
		padding: 2rem;
		box-shadow: var(--shadow-sm);
		border: 1px solid var(--color-grey-100);
		transition: all 300ms var(--ease-out);
	}

	.why-different__card:hover {
		box-shadow: var(--shadow-lg);
	}

	.why-different__icon-wrap {
		width: 3rem;
		height: 3rem;
		background-color: rgba(15, 164, 175, 0.1);
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 1.5rem;
	}

	.why-different__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin-bottom: 0.75rem;
		font-family: var(--font-heading);
	}

	.why-different__desc {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
	}
</style>
