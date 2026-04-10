<script lang="ts">
	import { onMount } from 'svelte';
	import { sampleAlert } from '$lib/data/alerts';
	import { gsap } from 'gsap';
	import { EASE, DURATION, isReducedMotion } from '$lib/utils/animations';
	import MiniChart from '$lib/components/charts/MiniChart.svelte';

	interface Props {
		delay?: number;
	}

	let { delay = 0.8 }: Props = $props();

	let cardRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!cardRef) return;
		const el = cardRef;
		const reduced = isReducedMotion();

		gsap.set(el, {
			opacity: 0,
			x: reduced ? 0 : 60,
			scale: reduced ? 1 : 0.92,
			filter: reduced ? 'none' : 'blur(12px)',
			willChange: 'transform, opacity, filter'
		});

		const ctx = gsap.context(() => {
			gsap.to(el, {
				opacity: 1,
				x: 0,
				scale: 1,
				filter: 'blur(0px)',
				duration: reduced ? 0.01 : DURATION.cinematic,
				delay: reduced ? 0 : delay,
				ease: reduced ? 'none' : EASE.cinematic,
				onComplete() {
					gsap.set(el, { willChange: 'auto', clearProps: 'filter,transform' });
				}
			});
		}, el);

		return () => ctx.revert();
	});
</script>

<div bind:this={cardRef} class="alert-card">
	<!-- Header -->
	<div class="alert-card__header">
		<span class="alert-card__label">Sample Alert</span>
		<div class="alert-card__status">
			<span class="alert-card__status-dot animate-pulse"></span>
			<span class="alert-card__status-text">Live Format</span>
		</div>
	</div>

	<!-- Ticker -->
	<div class="alert-card__ticker">{sampleAlert.ticker}</div>

	<!-- Mini Chart -->
	<div class="alert-card__chart">
		<MiniChart ticker={sampleAlert.ticker} trend="up" height={168} days={14} />
	</div>

	<!-- Data Rows -->
	<div class="alert-card__rows">
		<div class="alert-card__row">
			<span class="alert-card__row-label">Entry Zone</span>
			<span class="alert-card__row-value alert-card__row-value--teal">{sampleAlert.entryZone}</span>
		</div>
		<div class="alert-card__row">
			<span class="alert-card__row-label">Invalidation</span>
			<span class="alert-card__row-value alert-card__row-value--red"
				>{sampleAlert.invalidation}</span
			>
		</div>
		<div class="alert-card__row">
			<span class="alert-card__row-label">Profit Zone 1</span>
			<span class="alert-card__row-value alert-card__row-value--green"
				>{sampleAlert.profitZones[0]}</span
			>
		</div>
		<div class="alert-card__row">
			<span class="alert-card__row-label">Profit Zone 2</span>
			<span class="alert-card__row-value alert-card__row-value--green"
				>{sampleAlert.profitZones[1]}</span
			>
		</div>
		<div class="alert-card__row">
			<span class="alert-card__row-label">Profit Zone 3</span>
			<span class="alert-card__row-value alert-card__row-value--green"
				>{sampleAlert.profitZones[2]}</span
			>
		</div>
	</div>

	<!-- Notes -->
	<div class="alert-card__notes">
		<p class="alert-card__notes-text">{sampleAlert.notes}</p>
	</div>
</div>

<style>
	.alert-card {
		max-width: 28rem;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.1);
		background-color: rgba(255, 255, 255, 0.05);
		padding: 1.5rem;
		backdrop-filter: blur(4px);
	}

	.alert-card__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}

	.alert-card__label {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.alert-card__status {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.alert-card__status-dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: var(--radius-full);
		background-color: var(--color-green);
	}

	.alert-card__status-text {
		color: var(--color-green);
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
	}

	.alert-card__ticker {
		margin-bottom: 0.75rem;
		font-family: var(--font-ui);
		font-size: 1.875rem;
		font-weight: var(--w-bold);
		letter-spacing: -0.025em;
		color: var(--color-white);
		font-variant-numeric: tabular-nums;
	}

	.alert-card__chart {
		margin-bottom: 1rem;
		border-radius: var(--radius-md);
		overflow: hidden;
		background: rgba(0, 0, 0, 0.2);
	}

	.alert-card__rows {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.alert-card__row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.alert-card__row-label {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.alert-card__row-value {
		font-family: var(--font-ui);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		font-variant-numeric: tabular-nums;
	}

	.alert-card__row-value--teal {
		color: var(--color-teal-light);
	}
	.alert-card__row-value--red {
		color: var(--color-red);
	}
	.alert-card__row-value--green {
		color: var(--color-green);
	}

	.alert-card__notes {
		margin-top: 1rem;
		background-color: rgba(15, 164, 175, 0.1);
		border-left: 2px solid var(--color-teal);
		border-radius: 0 var(--radius-lg) var(--radius-lg) 0;
		padding: 0.75rem;
	}

	.alert-card__notes-text {
		color: var(--color-grey-200);
		font-size: var(--fs-xs);
		line-height: 1.65;
	}
</style>
