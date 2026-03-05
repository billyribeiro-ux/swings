<script lang="ts">
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';

	const audience = [
		'Traders who want <strong>simple, followable alerts</strong>',
		"People who don't have time to learn advanced strategies",
		'Traders who want <strong>early entries</strong>, not late chases',
		'Anyone who wants guidance without sitting in a live room'
	];
</script>

<section class="who-its-for">
	<div class="who-its-for__container">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Who This Is For"
				title="If You Want Clarity Without Complexity,<br/>This Was Built for You"
			/>

			<div class="who-its-for__grid">
				{#each audience as item, i}
					<div class="reveal-item who-its-for__item" style="transition-delay: {i * 0.1}s">
						<CaretRight size={20} weight="bold" color="#0FA4AF" class="who-its-for__icon" />
						<p class="who-its-for__text">{@html item}</p>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	.who-its-for {
		padding: 5rem 0;
		background-color: var(--color-off-white);
	}

	@media (min-width: 1024px) {
		.who-its-for {
			padding: 8rem 0;
		}
	}

	.who-its-for__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.who-its-for__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.who-its-for__container {
			padding: 0 2rem;
		}
	}

	.who-its-for__grid {
		display: grid;
		gap: 1rem;
		max-width: 56rem;
		margin: 0 auto;
	}

	@media (min-width: 768px) {
		.who-its-for__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.who-its-for__item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		padding: 1.25rem;
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-grey-200);
		background-color: var(--color-white);
	}

	:global(.who-its-for__icon) {
		flex-shrink: 0;
		margin-top: 0.125rem;
	}

	.who-its-for__text {
		color: var(--color-grey-800);
		text-align: left;
	}
</style>
