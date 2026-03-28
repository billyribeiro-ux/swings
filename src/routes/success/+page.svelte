<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicCascade, EASE, DURATION } from '$lib/utils/animations';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import Envelope from 'phosphor-svelte/lib/Envelope';
	import BrowsersIcon from 'phosphor-svelte/lib/Browsers';
	import CalendarCheck from 'phosphor-svelte/lib/CalendarCheck';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Button from '$lib/components/ui/Button.svelte';

	let sessionId = $state('');
	let containerRef: HTMLElement | undefined = $state();

	onMount(() => {
		sessionId = $page.url.searchParams.get('session_id') || '';

		if (!containerRef) return;

		const ctx = gsap.context(() => {
			createCinematicCascade(
				containerRef!,
				[
					{
						selector: '.success-icon',
						duration: DURATION.fast,
						ease: EASE.elastic,
						y: 16,
						blur: 8,
						scale: 0.7,
						overlap: 0
					},
					{
						selector: '.success-title',
						duration: DURATION.cinematic,
						ease: EASE.cinematic,
						y: 36,
						blur: 10,
						scale: 0.95,
						overlap: 0.55
					},
					{
						selector: '.success-subtitle',
						duration: DURATION.slow,
						ease: EASE.soft,
						y: 28,
						blur: 8,
						scale: 0.98,
						overlap: 0.6
					},
					{
						selector: '.success-steps',
						duration: DURATION.slow,
						ease: EASE.cinematic,
						y: 40,
						blur: 8,
						scale: 0.96,
						overlap: 0.55
					},
					{
						selector: '.success-cta',
						duration: DURATION.normal,
						ease: EASE.snappy,
						y: 24,
						blur: 6,
						scale: 0.96,
						overlap: 0.5
					}
				],
				{ delay: 0.2 }
			);
		}, containerRef as HTMLElement);

		return () => ctx.revert();
	});

	const steps = [
		{
			icon: Envelope,
			title: 'Check Your Email',
			desc: "You'll receive login credentials and onboarding instructions within the next few minutes."
		},
		{
			icon: BrowsersIcon,
			title: 'Access Your Dashboard',
			desc: 'Log in to view your member area, past watchlists, and educational resources.'
		},
		{
			icon: CalendarCheck,
			title: 'Get Your First Watchlist',
			desc: 'Every Sunday night at 8 PM ET, your weekly watchlist will be delivered via email and SMS.'
		}
	];
</script>

<svelte:head>
	<title>Welcome to Explosive Swings!</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<div bind:this={containerRef} class="success-page">
	<div class="success-page__inner">
		<!-- Success Icon -->
		<div class="success-icon success-page__icon-wrap">
			<div class="success-page__icon-circle">
				<CheckCircle size={48} weight="fill" color="#22B573" />
			</div>
		</div>

		<!-- Title -->
		<h1 class="success-title success-page__title">Welcome to Explosive Swings!</h1>

		<!-- Subtitle -->
		<p class="success-subtitle success-page__subtitle">
			Your subscription is confirmed. Check your email for access details and your first Sunday
			night watchlist.
		</p>

		<!-- Steps -->
		<div class="success-steps success-page__steps">
			<div class="success-page__steps-header">
				<h2 class="success-page__steps-title">What Happens Next?</h2>
			</div>

			<div class="success-page__steps-list">
				{#each steps as step, i (step.title)}
					<div class="success-page__step">
						<div class="success-page__step-icon">
							<step.icon size={20} weight="duotone" color="#15C5D1" />
						</div>
						<div>
							<h3 class="success-page__step-title">{step.title}</h3>
							<p class="success-page__step-desc">{step.desc}</p>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- CTA -->
		<div class="success-cta success-page__cta">
			<Button variant="primary" href="/">
				Back to Home
				<ArrowRight size={18} weight="bold" />
			</Button>
		</div>

		{#if sessionId}
			<p class="success-page__session">
				Session: {sessionId}
			</p>
		{/if}
	</div>
</div>

<style>
	.success-page {
		display: flex;
		min-height: 100vh;
		align-items: center;
		justify-content: center;
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
		padding: 6rem 1rem 7rem;
	}

	.success-page__inner {
		width: 100%;
		max-width: 42rem;
		margin: 0 auto;
		text-align: center;
	}

	.success-page__icon-wrap {
		display: flex;
		justify-content: center;
		margin-bottom: 2rem;
	}

	.success-page__icon-circle {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 5rem;
		height: 5rem;
		border-radius: var(--radius-full);
		background-color: rgba(34, 181, 115, 0.15);
		box-shadow: 0 0 0 4px rgba(34, 181, 115, 0.1);
	}

	@media (min-width: 640px) {
		.success-page__icon-circle {
			width: 6rem;
			height: 6rem;
		}
	}

	.success-page__title {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 1rem;
	}

	@media (min-width: 640px) {
		.success-page__title {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 768px) {
		.success-page__title {
			font-size: clamp(2.5rem, 5vw, 3rem);
		}
	}

	.success-page__subtitle {
		color: var(--color-grey-300);
		font-size: 1rem;
		line-height: 1.65;
		margin-bottom: 2.5rem;
	}

	@media (min-width: 640px) {
		.success-page__subtitle {
			font-size: var(--fs-lg);
		}
	}

	.success-page__steps {
		margin-bottom: 2.5rem;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.1);
		background-color: rgba(255, 255, 255, 0.04);
		text-align: left;
		backdrop-filter: blur(4px);
	}

	.success-page__steps-header {
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		padding: 1.25rem 1.5rem;
	}

	@media (min-width: 640px) {
		.success-page__steps-header {
			padding: 1.25rem 2rem;
		}
	}

	.success-page__steps-title {
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	@media (min-width: 640px) {
		.success-page__steps-title {
			font-size: var(--fs-xl);
		}
	}

	.success-page__steps-list > * + * {
		border-top: 1px solid rgba(255, 255, 255, 0.08);
	}

	.success-page__step {
		display: flex;
		gap: 1rem;
		padding: 1.25rem 1.5rem;
	}

	@media (min-width: 640px) {
		.success-page__step {
			padding: 1.5rem 2rem;
		}
	}

	.success-page__step-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		flex-shrink: 0;
		border-radius: var(--radius-xl);
		background-color: rgba(15, 164, 175, 0.15);
	}

	.success-page__step-title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin-bottom: 0.25rem;
	}

	@media (min-width: 640px) {
		.success-page__step-title {
			font-size: 1rem;
		}
	}

	.success-page__step-desc {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		line-height: 1.65;
	}

	@media (min-width: 640px) {
		.success-page__step-desc {
			font-size: var(--fs-sm);
		}
	}

	.success-page__cta {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.success-page__cta {
			flex-direction: row;
			justify-content: center;
		}
	}

	.success-page__session {
		color: var(--color-grey-600);
		margin-top: 2.5rem;
		font-size: var(--fs-xs);
	}
</style>
