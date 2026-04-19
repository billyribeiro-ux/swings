<script lang="ts">
	import Button from '$lib/components/ui/Button.svelte';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import { ctaImpression, trackCtaEvent } from '$lib/analytics/cta';

	const FINAL_CTA_ID = 'final_get_instant_access';
	import { gsap } from 'gsap';
	import {
		createCinematicReveal,
		createGlowBreathing,
		EASE,
		DURATION
	} from '$lib/utils/animations';

	let sectionRef: HTMLElement | undefined = $state();
	let glowRef: HTMLElement | undefined = $state();

	$effect(() => {
		if (!sectionRef || !glowRef) return;
		const section = sectionRef;
		const glow = glowRef;

		const contentEls = section.querySelectorAll('.final-cta-content > *');

		const ctx = gsap.context(() => {
			createGlowBreathing(glow, { scale: 1.15, opacity: 0.55, duration: 8 });

			createCinematicReveal({
				targets: contentEls,
				trigger: section,
				y: 36,
				blur: 10,
				scale: 0.95,
				duration: DURATION.cinematic,
				stagger: 0.14,
				ease: EASE.cinematic,
				start: 'top 78%'
			});
		}, section);

		return () => ctx.revert();
	});
</script>

<section bind:this={sectionRef} class="final-cta">
	<!-- Background -->
	<div class="final-cta__bg"></div>

	<!-- Centered Glow Orb -->
	<div bind:this={glowRef} class="final-cta__glow"></div>

	<div class="final-cta-content final-cta__container">
		<h2 class="final-cta__heading">Trade with Clarity. Trade with Confidence.</h2>
		<p class="final-cta__desc">
			Get your weekly watchlist every Sunday night - detailed entries, targets, exits, and stops so
			you're prepared before the market opens.
		</p>
		<div {@attach ctaImpression({ ctaId: FINAL_CTA_ID })}>
			<Button
				variant="primary"
				href="/pricing/monthly"
				onclick={() => trackCtaEvent('click', FINAL_CTA_ID)}
			>
				Get Instant Access to Alerts
				<ArrowRightIcon size={20} weight="bold" />
			</Button>
		</div>
	</div>
</section>

<style>
	.final-cta {
		position: relative;
		overflow: hidden;
		padding: 5rem 0;
	}

	@media (min-width: 1024px) {
		.final-cta {
			padding: 8rem 0;
		}
	}

	.final-cta__bg {
		position: absolute;
		inset: 0;
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.final-cta__glow {
		pointer-events: none;
		position: absolute;
		top: 50%;
		left: 50%;
		width: 600px;
		height: 600px;
		transform: translate(-50%, -50%);
		border-radius: var(--radius-full);
		opacity: 0.4;
		background: radial-gradient(circle, rgba(15, 164, 175, 0.4) 0%, transparent 70%);
	}

	.final-cta__container {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
		text-align: center;
	}

	@media (min-width: 640px) {
		.final-cta__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.final-cta__container {
			padding: 0 2rem;
		}
	}

	.final-cta__heading {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 1.5rem;
	}

	@media (min-width: 768px) {
		.final-cta__heading {
			font-size: var(--fs-4xl);
		}
	}

	@media (min-width: 1024px) {
		.final-cta__heading {
			font-size: clamp(2.5rem, 4vw, 3rem);
		}
	}

	.final-cta__desc {
		color: var(--color-grey-300);
		max-width: 42rem;
		margin: 0 auto 2rem;
		font-size: var(--fs-lg);
	}
</style>
