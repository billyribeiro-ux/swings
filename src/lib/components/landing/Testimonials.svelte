<script lang="ts">
	import { gsap } from 'gsap';
	import { EASE, DURATION, isReducedMotion } from '$lib/utils/animations';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import StarIcon from 'phosphor-svelte/lib/StarIcon';
	import QuotesIcon from 'phosphor-svelte/lib/QuotesIcon';

	let containerRef: HTMLElement | undefined = $state();

	// Testimonial entries are intentionally empty until real, attributed
	// member quotes are sourced. The section renders an empty-state copy
	// in that case rather than fake quotes.
	//
	// TODO(testimonials): replace this constant with data fetched from a
	// future `GET /api/testimonials` endpoint backed by an admin-managed
	// `testimonials` table. The endpoint is not built yet — leaving the
	// shape here so wiring the fetch is a one-line change.
	interface Testimonial {
		name: string;
		role: string;
		avatar: string;
		rating: number;
		text: string;
		gradientFrom: string;
		gradientTo: string;
	}
	const testimonials: Testimonial[] = [];

	$effect(() => {
		if (!containerRef) return;
		const container = containerRef;

		const cards = container.querySelectorAll('.testimonial-card');
		const reduced = isReducedMotion();

		if (cards.length === 0) {
			return;
		}

		gsap.set(cards, {
			opacity: 0,
			y: reduced ? 0 : 50,
			scale: reduced ? 1 : 0.92,
			filter: reduced ? 'none' : 'blur(8px)',
			willChange: 'transform, opacity, filter'
		});

		const ctx = gsap.context(() => {
			gsap.to(cards, {
				opacity: 1,
				y: 0,
				scale: 1,
				filter: 'blur(0px)',
				duration: reduced ? 0.01 : DURATION.cinematic,
				stagger: reduced ? 0 : 0.15,
				ease: reduced ? 'none' : EASE.cinematic,
				scrollTrigger: { trigger: container, start: 'top 78%', once: true },
				onComplete: () => {
					gsap.set(cards, { willChange: 'auto', clearProps: 'filter,transform' });
				}
			});
		}, container);

		return () => ctx.revert();
	});
</script>

{#if testimonials.length > 0}
	<section bind:this={containerRef} class="testimonials">
		<div class="testimonials__container">
			<SectionHeader
				eyebrow="Testimonials"
				title="What Members Are Saying"
				subtitle="Real quotes from real members of Precision Options Signals."
			/>

			<div class="testimonials__grid">
				{#each testimonials as testimonial (testimonial.name)}
					<div class="testimonial-card">
						<!-- Quote icon -->
						<div
							class="testimonial-card__quote-icon"
							style="background: linear-gradient(to bottom right, {testimonial.gradientFrom}, {testimonial.gradientTo});"
						>
							<QuotesIcon size={24} weight="fill" color="white" />
						</div>

						<!-- Rating -->
						<div class="testimonial-card__rating">
							{#each Array.from({ length: testimonial.rating }) as _, j (j)}
								<StarIcon size={16} weight="fill" color="#D4A843" />
							{/each}
						</div>

						<!-- Text -->
						<p class="testimonial-card__text">
							"{testimonial.text}"
						</p>

						<!-- Author -->
						<div class="testimonial-card__author">
							<div
								class="testimonial-card__avatar"
								style="background: linear-gradient(to bottom right, {testimonial.gradientFrom}, {testimonial.gradientTo});"
							>
								{testimonial.avatar}
							</div>
							<div>
								<h4 class="testimonial-card__name">{testimonial.name}</h4>
								<p class="testimonial-card__role">{testimonial.role}</p>
							</div>
						</div>
					</div>
				{/each}
			</div>
		</div>
	</section>
{/if}

<style>
	.testimonials {
		background-color: var(--color-off-white);
		position: relative;
		overflow: hidden;
		padding: 4rem 0;
	}

	@media (min-width: 640px) {
		.testimonials {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.testimonials {
			padding: 8rem 0;
		}
	}

	.testimonials__container {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.testimonials__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.testimonials__container {
			padding: 0 2rem;
		}
	}

	.testimonials__grid {
		display: grid;
		gap: 2rem;
	}

	@media (min-width: 768px) {
		.testimonials__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (min-width: 1024px) {
		.testimonials__grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.testimonial-card {
		position: relative;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		background-color: var(--color-white);
		padding: 2rem;
		box-shadow: var(--shadow-sm);
		outline: 1px solid rgba(216, 220, 228, 0.6);
		outline-offset: -1px;
		transition:
			box-shadow 300ms var(--ease-out),
			outline-color 300ms var(--ease-out);
	}

	.testimonial-card:hover {
		box-shadow: var(--shadow-lg);
		outline-color: rgba(15, 164, 175, 0.2);
	}

	.testimonial-card__quote-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 3rem;
		height: 3rem;
		border-radius: var(--radius-xl);
		margin-bottom: 1.5rem;
		box-shadow: var(--shadow-sm);
	}

	.testimonial-card__rating {
		display: flex;
		gap: 0.125rem;
		margin-bottom: 1rem;
	}

	.testimonial-card__text {
		color: var(--color-grey-700);
		margin-bottom: 1.5rem;
		flex: 1;
		line-height: 1.65;
	}

	.testimonial-card__author {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		border-top: 1px solid var(--color-grey-100);
		padding-top: 1.5rem;
	}

	.testimonial-card__avatar {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		flex-shrink: 0;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.testimonial-card__name {
		color: var(--color-navy);
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
	}

	.testimonial-card__role {
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
	}
</style>
