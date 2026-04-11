<script lang="ts">
	import { sampleAlert } from '$lib/data/alerts';
	import { gsap } from 'gsap';
	import { EASE, DURATION, isReducedMotion } from '$lib/utils/animations';
	import MiniChart from '$lib/components/charts/MiniChart.svelte';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';
	import Lightbulb from 'phosphor-svelte/lib/Lightbulb';

	interface Props {
		delay?: number;
	}

	let { delay = 0.8 }: Props = $props();

	let cardRef: HTMLElement | undefined = $state();

	$effect(() => {
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

<article bind:this={cardRef} class="alert-card" aria-label="Sample stock alert preview">
	<div class="alert-card__accent" aria-hidden="true"></div>

	<header class="alert-card__header">
		<span class="alert-card__badge">Sample alert</span>
		<div class="alert-card__live">
			<span class="alert-card__live-ring" aria-hidden="true"></span>
			<span class="alert-card__live-dot"></span>
			<span class="alert-card__live-text">Live format</span>
		</div>
	</header>

	<div class="alert-card__hero">
		<div class="alert-card__symbol-block">
			<div class="alert-card__symbol-row">
				<span class="alert-card__ticker">{sampleAlert.ticker}</span>
				<span class="alert-card__chip">
					<TrendUp size={14} weight="bold" class="alert-card__chip-icon" aria-hidden="true" />
					Bullish
				</span>
			</div>
			<p class="alert-card__meta">US equities · Watchlist-style layout</p>
		</div>

		<div class="alert-card__chart-panel">
			<div class="alert-card__chart-meta">
				<span class="alert-card__chart-tag">14 sessions</span>
				<span class="alert-card__chart-dot" aria-hidden="true">·</span>
				<span class="alert-card__chart-tag">Daily candles</span>
			</div>
			<div class="alert-card__chart-inner">
				<MiniChart ticker={sampleAlert.ticker} trend="up" height={188} days={14} showcase={true} />
			</div>
		</div>
	</div>

	<div class="alert-card__body">
		<section class="alert-card__section" aria-labelledby="alert-pos-heading">
			<h3 id="alert-pos-heading" class="alert-card__section-title">Position</h3>
			<div class="alert-card__rows">
				<div class="alert-card__row">
					<span class="alert-card__row-label">Entry zone</span>
					<span class="alert-card__row-value alert-card__row-value--teal"
						>{sampleAlert.entryZone}</span
					>
				</div>
				<div class="alert-card__row">
					<span class="alert-card__row-label">Invalidation</span>
					<span class="alert-card__row-value alert-card__row-value--red"
						>{sampleAlert.invalidation}</span
					>
				</div>
			</div>
		</section>

		<section class="alert-card__section" aria-labelledby="alert-tg-heading">
			<h3 id="alert-tg-heading" class="alert-card__section-title">Profit targets</h3>
			<div class="alert-card__rows">
				<div class="alert-card__row">
					<span class="alert-card__row-label">Target 1</span>
					<span class="alert-card__row-value alert-card__row-value--green"
						>{sampleAlert.profitZones[0]}</span
					>
				</div>
				<div class="alert-card__row">
					<span class="alert-card__row-label">Target 2</span>
					<span class="alert-card__row-value alert-card__row-value--green"
						>{sampleAlert.profitZones[1]}</span
					>
				</div>
				<div class="alert-card__row">
					<span class="alert-card__row-label">Target 3</span>
					<span class="alert-card__row-value alert-card__row-value--green"
						>{sampleAlert.profitZones[2]}</span
					>
				</div>
			</div>
		</section>
	</div>

	<div class="alert-card__notes">
		<Lightbulb size={18} weight="duotone" class="alert-card__notes-icon" aria-hidden="true" />
		<p class="alert-card__notes-text">{sampleAlert.notes}</p>
	</div>
</article>

<style>
	.alert-card {
		position: relative;
		width: 100%;
		max-width: 30rem;
		overflow: hidden;
		border-radius: 1.25rem;
		border: 1px solid rgba(255, 255, 255, 0.14);
		background: linear-gradient(
			155deg,
			rgba(15, 23, 42, 0.92) 0%,
			rgba(15, 23, 42, 0.72) 45%,
			rgba(15, 30, 55, 0.85) 100%
		);
		box-shadow:
			0 0 0 1px rgba(0, 0, 0, 0.35) inset,
			0 28px 56px -16px rgba(0, 0, 0, 0.55),
			0 0 100px -30px rgba(15, 164, 175, 0.35);
		backdrop-filter: blur(20px) saturate(1.25);
		padding: 0;
	}

	.alert-card__accent {
		height: 3px;
		width: 100%;
		background: linear-gradient(
			90deg,
			transparent 0%,
			var(--color-teal, #0fa4af) 20%,
			#5eead4 50%,
			var(--color-teal, #0fa4af) 80%,
			transparent 100%
		);
		opacity: 0.95;
	}

	.alert-card__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 1rem 1.35rem 0.75rem;
	}

	.alert-card__badge {
		display: inline-flex;
		align-items: center;
		padding: 0.35rem 0.75rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(255, 255, 255, 0.12);
		background: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}

	.alert-card__live {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 0.45rem;
		padding: 0.35rem 0.7rem 0.35rem 0.5rem;
		border-radius: var(--radius-full);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.35);
	}

	.alert-card__live-ring {
		position: absolute;
		inset: -2px;
		border-radius: inherit;
		border: 1px solid rgba(34, 181, 115, 0.2);
		pointer-events: none;
	}

	.alert-card__live-dot {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: var(--radius-full);
		background: var(--color-green, #22b573);
		box-shadow: 0 0 10px rgba(34, 181, 115, 0.85);
		animation: live-pulse 2s ease-in-out infinite;
	}

	@keyframes live-pulse {
		0%,
		100% {
			opacity: 1;
			transform: scale(1);
		}
		50% {
			opacity: 0.85;
			transform: scale(1.15);
		}
	}

	.alert-card__live-text {
		color: #86efac;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.02em;
	}

	.alert-card__hero {
		padding: 0 1.35rem 1rem;
	}

	.alert-card__symbol-block {
		margin-bottom: 1rem;
	}

	.alert-card__symbol-row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.65rem;
		margin-bottom: 0.35rem;
	}

	.alert-card__ticker {
		font-family: var(--font-heading);
		font-size: clamp(2rem, 5vw, 2.35rem);
		font-weight: var(--w-extrabold);
		letter-spacing: -0.03em;
		line-height: 1;
		color: var(--color-white);
		font-variant-numeric: tabular-nums lining-nums;
	}

	.alert-card__chip {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.25rem 0.6rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		letter-spacing: 0.02em;
		color: #86efac;
		background: linear-gradient(135deg, rgba(34, 181, 115, 0.2), rgba(15, 164, 175, 0.15));
		border: 1px solid rgba(34, 181, 115, 0.35);
	}

	:global(.alert-card__chip-icon) {
		flex-shrink: 0;
		color: #86efac;
	}

	.alert-card__meta {
		margin: 0;
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
		letter-spacing: 0.02em;
	}

	.alert-card__chart-panel {
		position: relative;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.08);
		background:
			linear-gradient(180deg, rgba(15, 164, 175, 0.09) 0%, transparent 42%),
			radial-gradient(ellipse 90% 80% at 50% 0%, rgba(15, 164, 175, 0.12), transparent 55%),
			rgba(0, 0, 0, 0.28);
		box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06);
		overflow: hidden;
	}

	.alert-card__chart-meta {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.35rem;
		padding: 0.65rem 0.85rem 0;
		font-size: 0.65rem;
		font-weight: var(--w-semibold);
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--color-grey-500);
	}

	.alert-card__chart-dot {
		opacity: 0.5;
		user-select: none;
	}

	.alert-card__chart-tag {
		color: rgba(148, 163, 184, 0.95);
	}

	.alert-card__chart-inner {
		padding: 0 0.35rem 0.5rem;
	}

	.alert-card__body {
		display: flex;
		flex-direction: column;
		gap: 1.15rem;
		padding: 0 1.35rem 1.25rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		background: linear-gradient(180deg, transparent, rgba(0, 0, 0, 0.15));
	}

	.alert-card__section-title {
		margin: 0 0 0.6rem;
		font-family: var(--font-ui);
		font-size: 0.65rem;
		font-weight: var(--w-bold);
		line-height: 1.3;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--color-grey-500);
	}

	.alert-card__rows {
		display: flex;
		flex-direction: column;
		gap: 0;
		border-radius: var(--radius-md);
		border: 1px solid rgba(255, 255, 255, 0.06);
		background: rgba(0, 0, 0, 0.2);
		overflow: hidden;
	}

	.alert-card__row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.65rem 0.85rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.alert-card__row:last-child {
		border-bottom: none;
	}

	.alert-card__row-label {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
	}

	.alert-card__row-value {
		font-family: var(--font-ui);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		font-variant-numeric: tabular-nums lining-nums;
		text-align: right;
	}

	.alert-card__row-value--teal {
		color: var(--color-teal-light, #5eead4);
	}

	.alert-card__row-value--red {
		color: #fb7185;
	}

	.alert-card__row-value--green {
		color: #86efac;
	}

	.alert-card__notes {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
		margin: 0 1.35rem 1.35rem;
		padding: 1rem 1rem 1rem 0.95rem;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(15, 164, 175, 0.25);
		background: linear-gradient(120deg, rgba(15, 164, 175, 0.12) 0%, rgba(15, 23, 42, 0.5) 100%);
	}

	:global(.alert-card__notes-icon) {
		flex-shrink: 0;
		margin-top: 0.1rem;
		color: var(--color-teal-light, #5eead4);
		opacity: 0.95;
	}

	.alert-card__notes-text {
		margin: 0;
		color: var(--color-grey-200);
		font-size: var(--fs-sm);
		line-height: 1.65;
		letter-spacing: -0.01em;
	}

	@media (prefers-reduced-motion: reduce) {
		.alert-card__live-dot {
			animation: none;
		}
	}
</style>
