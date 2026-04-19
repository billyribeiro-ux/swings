<script lang="ts">
	import { browser } from '$app/environment';
	import { gsap } from 'gsap';
	import Calendar from 'phosphor-svelte/lib/Calendar';
	import Clock from 'phosphor-svelte/lib/Clock';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import { createCinematicReveal, DURATION, EASE, hoverTilt } from '$lib/utils/animations';
	import {
		EASTERN_TZ,
		getMarketCountdown,
		getNextMarketOpen,
		getNextWatchlistDelivery,
		getWatchlistCountdown,
		pad2
	} from '$lib/utils/tradingSchedule';

	let containerRef: HTMLElement | undefined = $state();
	let ready = $state(false);

	let wDays = $state(0);
	let wHours = $state(0);
	let wMinutes = $state(0);
	let wSeconds = $state(0);
	let mHours = $state(0);
	let mMinutes = $state(0);
	let mSeconds = $state(0);
	let deliveryWhen = $state('');
	let marketWhen = $state('');

	const etFormatter = new Intl.DateTimeFormat('en-US', {
		dateStyle: 'medium',
		timeStyle: 'short',
		timeZone: EASTERN_TZ
	});

	function tick() {
		const now = new Date();
		const nextDelivery = getNextWatchlistDelivery(now);
		const nextOpen = getNextMarketOpen(now);
		const w = getWatchlistCountdown(now, nextDelivery);
		const m = getMarketCountdown(now, nextOpen);
		wDays = w.days;
		wHours = w.hours;
		wMinutes = w.minutes;
		wSeconds = w.seconds;
		mHours = m.hours;
		mMinutes = m.minutes;
		mSeconds = m.seconds;
		deliveryWhen = `${etFormatter.format(nextDelivery)} ET`;
		marketWhen = `${etFormatter.format(nextOpen)} ET`;
	}

	$effect(() => {
		if (!browser) return;
		tick();
		ready = true;
		const id = setInterval(tick, 1000);
		return () => clearInterval(id);
	});

	$effect(() => {
		if (!containerRef || !browser) return;
		const container = containerRef;
		const cards = container.querySelectorAll<HTMLElement>('.schedule-countdowns__card');
		if (!cards.length) return;

		const ctx = gsap.context(() => {
			createCinematicReveal({
				targets: cards,
				trigger: container,
				y: 28,
				blur: 6,
				scale: 0.98,
				duration: DURATION.slow,
				stagger: 0.14,
				ease: EASE.cinematic,
				start: 'top 82%'
			});
		}, container);

		return () => ctx.revert();
	});
</script>

<section
	bind:this={containerRef}
	class="schedule-countdowns"
	aria-label="Watchlist delivery and US market open countdowns"
>
	<div class="schedule-countdowns__container">
		<SectionHeader
			eyebrow="Timing"
			title="Your Week on Eastern Time"
			subtitle="Watchlists drop Sunday evening; the US cash session opens Monday through Friday."
		/>

		<div class="schedule-countdowns__grid">
			<article class="schedule-countdowns__card">
				<div class="schedule-countdowns__icon-wrap" aria-hidden="true">
					<Calendar size={26} weight="duotone" color="#0FA4AF" />
				</div>
				<p class="schedule-countdowns__eyebrow hero-eyebrow">Weekly delivery</p>
				<h3 class="schedule-countdowns__title">Next watchlist</h3>
				<p class="schedule-countdowns__sub">Sundays at 6:00 PM Eastern Time</p>

				<div class="schedule-countdowns__chips" role="timer" aria-label="Time until next watchlist delivery">
					{#if ready}
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value">{wDays}</span>
							<span class="schedule-countdowns__chip-label">Days</span>
						</div>
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value">{pad2(wHours)}</span>
							<span class="schedule-countdowns__chip-label">Hours</span>
						</div>
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value">{pad2(wMinutes)}</span>
							<span class="schedule-countdowns__chip-label">Minutes</span>
						</div>
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value">{pad2(wSeconds)}</span>
							<span class="schedule-countdowns__chip-label">Seconds</span>
						</div>
					{:else}
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value schedule-countdowns__placeholder">—</span>
							<span class="schedule-countdowns__chip-label">Days</span>
						</div>
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value schedule-countdowns__placeholder">—</span>
							<span class="schedule-countdowns__chip-label">Hours</span>
						</div>
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value schedule-countdowns__placeholder">—</span>
							<span class="schedule-countdowns__chip-label">Minutes</span>
						</div>
						<div class="schedule-countdowns__chip">
							<span class="schedule-countdowns__chip-value schedule-countdowns__placeholder">—</span>
							<span class="schedule-countdowns__chip-label">Seconds</span>
						</div>
					{/if}
				</div>

				<p class="schedule-countdowns__when">
					<span class="schedule-countdowns__when-label">Scheduled for</span>
					<span class="schedule-countdowns__when-value">{ready ? deliveryWhen : '—'}</span>
				</p>
			</article>

			<article class="schedule-countdowns__card" {@attach hoverTilt({ maxTilt: 4, scale: 1.01 })}>
				<div class="schedule-countdowns__icon-wrap" aria-hidden="true">
					<Clock size={26} weight="duotone" color="#0FA4AF" />
				</div>
				<p class="schedule-countdowns__eyebrow hero-eyebrow">US session</p>
				<h3 class="schedule-countdowns__title">Market opens</h3>
				<p class="schedule-countdowns__sub">Mon–Fri · 9:30 AM Eastern Time</p>

				<div
					class="schedule-countdowns__clock kpi-value"
					aria-hidden="true"
				>
					{#if ready}
						{pad2(mHours)}:{pad2(mMinutes)}:{pad2(mSeconds)}
					{:else}
						<span class="schedule-countdowns__placeholder">—:—:—</span>
					{/if}
				</div>

				<p class="schedule-countdowns__sr">
					{#if ready}
						Countdown to the next US cash session open at {marketWhen}. Hours may exceed twenty-four
						when the next session is more than a day away.
					{:else}
						Loading countdown to the next market open.
					{/if}
				</p>

				<p class="schedule-countdowns__when">
					<span class="schedule-countdowns__when-label">Next open</span>
					<span class="schedule-countdowns__when-value">{ready ? marketWhen : '—'}</span>
				</p>

				<p class="schedule-countdowns__note">
					Regular session hours; exchange holidays are not reflected in this timer.
				</p>
			</article>
		</div>
	</div>
</section>

<style>
	.schedule-countdowns {
		padding: 5rem 0;
		background-color: var(--color-white);
	}

	@media (min-width: 1024px) {
		.schedule-countdowns {
			padding: 6rem 0;
		}
	}

	.schedule-countdowns__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.schedule-countdowns__container {
			padding: 0 1.5rem;
		}
	}

	@media (min-width: 1024px) {
		.schedule-countdowns__container {
			padding: 0 2rem;
		}
	}

	.schedule-countdowns__grid {
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 900px) {
		.schedule-countdowns__grid {
			grid-template-columns: 1fr 1fr;
			gap: 2rem;
			align-items: stretch;
		}
	}

	.schedule-countdowns__card {
		position: relative;
		background: linear-gradient(150deg, #ffffff 0%, #f4f7fb 100%);
		border-radius: var(--radius-xl);
		padding: 2.25rem;
		box-shadow: 
			0 4px 6px -1px rgba(11, 29, 58, 0.04), 
			0 2px 4px -2px rgba(11, 29, 58, 0.03),
			inset 0 1px 0 rgba(255, 255, 255, 1);
		border: 1px solid rgba(226, 232, 240, 0.9);
		transition:
			box-shadow 400ms var(--ease-out),
			border-color 400ms var(--ease-out);
	}

	.schedule-countdowns__card:hover {
		box-shadow: 
			0 20px 25px -5px rgba(11, 29, 58, 0.08), 
			0 8px 10px -6px rgba(11, 29, 58, 0.04),
			inset 0 1px 0 rgba(255, 255, 255, 1);
		border-color: rgba(203, 213, 225, 0.8);
	}

	.schedule-countdowns__icon-wrap {
		width: 3rem;
		height: 3rem;
		background-color: rgba(15, 164, 175, 0.1);
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 1.25rem;
	}

	.schedule-countdowns__eyebrow {
		margin-bottom: 0.5rem;
	}

	.schedule-countdowns__title {
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin-bottom: 0.5rem;
	}

	@media (min-width: 640px) {
		.schedule-countdowns__title {
			font-size: var(--fs-2xl);
		}
	}

	.schedule-countdowns__sub {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.6;
		margin-bottom: 1.5rem;
	}

	.schedule-countdowns__chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}

	.schedule-countdowns__chip {
		flex: 1;
		min-width: 4.5rem;
		background: var(--color-white);
		border: 1px solid var(--color-grey-200);
		border-radius: var(--radius-lg);
		padding: 0.75rem 0.5rem;
		text-align: center;
	}

	.schedule-countdowns__chip-value {
		display: block;
		font-family: var(--font-ui);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		font-variant-numeric: tabular-nums lining-nums;
		color: var(--color-navy);
		line-height: 1.2;
	}

	.schedule-countdowns__chip-label {
		font-size: var(--fs-2xs);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: var(--ls-wide);
		color: var(--color-grey-500);
		margin-top: 0.25rem;
		display: block;
	}

	.schedule-countdowns__clock {
		font-size: clamp(2rem, 6vw, 3rem);
		font-weight: var(--w-extrabold);
		font-variant-numeric: tabular-nums lining-nums;
		color: var(--color-navy);
		letter-spacing: 0.02em;
		margin-bottom: 1.25rem;
		line-height: 1.1;
		background: linear-gradient(135deg, var(--color-navy) 0%, var(--color-teal-dark, #0d8a94) 100%);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.schedule-countdowns__placeholder {
		color: var(--color-grey-400);
	}

	.schedule-countdowns__when {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-600);
	}

	.schedule-countdowns__when-label {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: var(--ls-wide);
	}

	.schedule-countdowns__when-value {
		font-weight: var(--w-semibold);
		color: var(--color-navy);
	}

	.schedule-countdowns__note {
		margin-top: 1rem;
		font-size: var(--fs-2xs);
		color: var(--color-grey-500);
		line-height: 1.5;
	}

	.schedule-countdowns__sr {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
</style>
