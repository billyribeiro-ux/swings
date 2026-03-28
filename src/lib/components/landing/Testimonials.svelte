<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { EASE, DURATION, isReducedMotion } from '$lib/utils/animations';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import Star from 'phosphor-svelte/lib/Star';
	import Quotes from 'phosphor-svelte/lib/Quotes';

	let containerRef: HTMLElement | undefined = $state();

	const testimonials = [
		{
			name: 'Michael Chen',
			role: 'Full-Time Trader',
			avatar: 'MC',
			rating: 5,
			text: "Billy's watchlists have completely transformed my trading. The entry zones are spot-on, and the risk management is exactly what I needed. I've gone from guessing to executing with confidence.",
			gradientFrom: 'var(--color-teal)',
			gradientTo: 'var(--color-teal-light)'
		},
		{
			name: 'Sarah Martinez',
			role: 'Portfolio Manager',
			avatar: 'SM',
			rating: 5,
			text: "I've tried dozens of trading services, and Explosive Swings is the only one I trust. No fluff, no spam-just high-quality setups delivered every Sunday night. The Discord community is gold.",
			gradientFrom: 'var(--color-gold)',
			gradientTo: 'var(--color-gold-light)'
		},
		{
			name: 'David Thompson',
			role: 'Options Trader',
			avatar: 'DT',
			rating: 5,
			text: 'The courses alone are worth 10x the subscription price. Billy breaks down complex strategies in a way that actually makes sense. My win rate has improved dramatically since joining.',
			gradientFrom: 'var(--color-deep-blue)',
			gradientTo: 'var(--color-teal)'
		}
	];

	onMount(() => {
		if (!containerRef) return;

		const cards = containerRef.querySelectorAll('.testimonial-card');
		const stats = containerRef.querySelectorAll('.stat-item');
		const reduced = isReducedMotion();

		gsap.set(cards, {
			opacity: 0,
			y: reduced ? 0 : 50,
			scale: reduced ? 1 : 0.92,
			filter: reduced ? 'none' : 'blur(8px)',
			willChange: 'transform, opacity, filter'
		});
		gsap.set(stats, {
			opacity: 0,
			y: reduced ? 0 : 24,
			scale: reduced ? 1 : 0.95,
			filter: reduced ? 'none' : 'blur(4px)',
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
				scrollTrigger: { trigger: containerRef, start: 'top 78%', once: true },
				onComplete: () => {
					gsap.set(cards, { willChange: 'auto', clearProps: 'filter,transform' });
				}
			});

			gsap.to(stats, {
				opacity: 1,
				y: 0,
				scale: 1,
				filter: 'blur(0px)',
				duration: reduced ? 0.01 : DURATION.normal,
				stagger: reduced ? 0 : 0.08,
				ease: reduced ? 'none' : EASE.snappy,
				scrollTrigger: {
					trigger: stats[0]?.parentElement,
					start: 'top 88%',
					once: true
				},
				onComplete: () => {
					gsap.set(stats, { willChange: 'auto', clearProps: 'filter,transform' });
				}
			});
		}, containerRef as HTMLElement);

		return () => ctx.revert();
	});
</script>

<section bind:this={containerRef} class="testimonials">
	<div class="testimonials__container">
		<SectionHeader
			eyebrow="Testimonials"
			title="Trusted by Thousands of Traders"
			subtitle="See what our members are saying about their experience with Explosive Swings."
		/>

		<div class="testimonials__grid">
			{#each testimonials as testimonial, i (testimonial.name)}
				<div class="testimonial-card">
					<!-- Quote icon -->
					<div
						class="testimonial-card__quote-icon"
						style="background: linear-gradient(to bottom right, {testimonial.gradientFrom}, {testimonial.gradientTo});"
					>
						<Quotes size={24} weight="fill" color="white" />
					</div>

					<!-- Rating -->
					<div class="testimonial-card__rating">
						{#each Array(testimonial.rating) as _, j}
							<Star size={16} weight="fill" color="#D4A843" />
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

		<!-- Social proof stats -->
		<div class="testimonials__stats">
			<div class="stat-item">
				<div class="stat-item__value">18,000+</div>
				<p class="stat-item__label">Active Traders</p>
			</div>
			<div class="stat-item">
				<div class="stat-item__value">4.9/5</div>
				<p class="stat-item__label">Average Rating</p>
			</div>
			<div class="stat-item">
				<div class="stat-item__value">95%</div>
				<p class="stat-item__label">Renewal Rate</p>
			</div>
		</div>
	</div>
</section>

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

	.testimonials__stats {
		margin-top: 4rem;
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 640px) {
		.testimonials__stats {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.stat-item {
		border: 1px solid var(--color-grey-200);
		border-radius: var(--radius-2xl);
		background-color: var(--color-white);
		padding: 1.5rem;
		text-align: center;
	}

	.stat-item__value {
		color: var(--color-teal);
		font-family: var(--font-heading);
		font-size: var(--fs-4xl);
		font-weight: var(--w-bold);
		margin-bottom: 0.5rem;
	}

	.stat-item__label {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
	}
</style>
