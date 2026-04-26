<script lang="ts">
	import { gsap } from 'gsap';
	import {
		createCinematicCascade,
		createGlowBreathing,
		EASE,
		DURATION
	} from '$lib/utils/animations';
	import Button from '$lib/components/ui/Button.svelte';
	import SampleAlertCard from './SampleAlertCard.svelte';
	import HeroChart from '$lib/components/charts/HeroChart.svelte';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import { ctaImpression, trackCtaEvent } from '$lib/analytics/cta';

	const HERO_PRIMARY_CTA = 'hero_get_instant_access';

	let heroRef: HTMLElement | undefined = $state();
	let glowRef: HTMLElement | undefined = $state();

	$effect(() => {
		if (!heroRef) return;

		const ctx = gsap.context(() => {
			createCinematicCascade(
				heroRef!,
				[
					{
						selector: '.hero-badge',
						duration: DURATION.fast,
						ease: EASE.snappy,
						y: 20,
						blur: 6,
						scale: 0.9,
						overlap: 0
					},
					{
						selector: '.hero-title',
						duration: DURATION.cinematic,
						ease: EASE.cinematic,
						y: 40,
						blur: 12,
						scale: 0.95,
						overlap: 0.6,
						lcpVisible: true
					},
					{
						selector: '.hero-subtitle',
						duration: DURATION.slow,
						ease: EASE.soft,
						y: 30,
						blur: 8,
						scale: 0.98,
						overlap: 0.65,
						lcpVisible: true
					},
					{
						selector: '.hero-actions',
						duration: DURATION.normal,
						ease: EASE.snappy,
						y: 24,
						blur: 6,
						scale: 0.96,
						overlap: 0.6
					},
					{
						selector: '.hero-trust',
						duration: DURATION.normal,
						ease: EASE.soft,
						y: 20,
						blur: 4,
						scale: 0.98,
						overlap: 0.55
					}
				],
				{ delay: 0.25 }
			);

			if (glowRef) {
				createGlowBreathing(glowRef, { scale: 1.15, opacity: 0.6, duration: 8 });
			}
		}, heroRef);

		return () => ctx.revert();
	});

	function scrollToHowItWorks() {
		const element = document.getElementById('how-it-works');
		if (element) {
			element.scrollIntoView({ behavior: 'smooth' });
		}
	}
</script>

<section bind:this={heroRef} class="hero">
	<!-- Background Chart -->
	<HeroChart height={500} days={60} />

	<!-- Background -->
	<div class="hero__bg"></div>

	<!-- Grid Overlay -->
	<div class="hero__grid-overlay"></div>

	<!-- Glow Orb -->
	<div bind:this={glowRef} class="hero__glow"></div>

	<div class="hero__container">
		<div class="hero__layout">
			<!-- Left Column -->
			<div class="hero__content">
				<!-- Eyebrow Badge -->
				<div class="hero-badge hero__badge">
					<span class="hero__badge-dot animate-pulse"></span>
					<span class="hero__badge-text">
						Weekly watchlist delivered every Sunday 6:00 PM ET
					</span>
				</div>

				<!-- Title -->
				<h1 class="hero-title hero__title">
					Simple, Early Stock Alerts <span class="hero__title-accent"
						>You Can Actually Use</span
					>
				</h1>

				<!-- Subtitle -->
				<p class="hero-subtitle hero__subtitle">
					Every Sunday night, get a detailed watchlist of 5–7 top stock picks with defined
					entries, targets, exits, and stops -- so you're ready before the market opens.
				</p>

				<!-- Actions -->
				<div class="hero-actions hero__actions">
					<div {@attach ctaImpression({ ctaId: HERO_PRIMARY_CTA })}>
						<Button
							variant="primary"
							href="/pricing/monthly"
							magnetic
							onclick={() => trackCtaEvent('click', HERO_PRIMARY_CTA)}
						>
							Get Instant Access
							<ArrowRightIcon size={20} weight="bold" />
						</Button>
					</div>
					<Button variant="ghost" onclick={scrollToHowItWorks} magnetic
						>See How It Works</Button
					>
				</div>

				<!-- Trust Line -->
				<div class="hero-trust hero__trust">
					<div class="hero__trust-avatar">BR</div>
					<p class="hero__trust-text">
						Created by <span class="hero__trust-name">Billy Ribeiro</span> - former lead trader
						at Simpler Trading, mentored by Goldman Sachs' Mark McGoldrick
					</p>
				</div>
			</div>

			<!-- Right Column -->
			<div class="hero__card-col">
				<SampleAlertCard delay={0.6} />
			</div>
		</div>
	</div>
</section>

<style>
	.hero {
		position: relative;
		min-height: 100vh;
		overflow: hidden;
		padding-top: 4rem;
	}

	.hero__bg {
		position: absolute;
		inset: 0;
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.hero__grid-overlay {
		position: absolute;
		inset: 0;
		opacity: 0.02;
		background-image:
			linear-gradient(to right, white 1px, transparent 1px),
			linear-gradient(to bottom, white 1px, transparent 1px);
		background-size: 60px 60px;
	}

	.hero__glow {
		pointer-events: none;
		position: absolute;
		top: 5rem;
		right: 2.5rem;
		width: 700px;
		height: 700px;
		border-radius: var(--radius-full);
		opacity: 0.6;
		background: radial-gradient(circle, rgba(15, 164, 175, 0.3) 0%, transparent 70%);
	}

	.hero__container {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 5rem 1rem;
	}

	@media (min-width: 640px) {
		.hero__container {
			padding: 5rem 1.5rem;
		}
	}

	@media (min-width: 1024px) {
		.hero__container {
			padding: 8rem 2rem;
		}
	}

	.hero__layout {
		display: grid;
		align-items: center;
		gap: 3rem;
	}

	@media (min-width: 1024px) {
		.hero__layout {
			grid-template-columns: 1fr 1fr;
			gap: 4rem;
		}
	}

	.hero__content {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.hero__badge {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(15, 164, 175, 0.3);
		background-color: rgba(15, 164, 175, 0.1);
		padding: 0.5rem 1rem;
		align-self: flex-start;
	}

	.hero__badge-dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: var(--radius-full);
		background-color: var(--color-teal);
	}

	.hero__badge-text {
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.hero__title {
		color: var(--color-white);
	}

	.hero__title-accent {
		color: var(--color-teal-light);
	}

	.hero__subtitle {
		color: var(--color-grey-300);
	}

	.hero__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.hero__trust {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding-top: 1rem;
	}

	.hero__trust-avatar {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
		color: var(--color-white);
		background: linear-gradient(135deg, #0fa4af 0%, #1a3a6b 100%);
		flex-shrink: 0;
	}

	.hero__trust-text {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.hero__trust-name {
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.hero__card-col {
		display: flex;
		justify-content: center;
	}

	@media (min-width: 1024px) {
		.hero__card-col {
			justify-content: flex-end;
		}
	}
</style>
