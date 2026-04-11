<script lang="ts">
	import { untrack } from 'svelte';
	import { generateTrendData } from '$lib/utils/chartData';
	import { ohlcToApexSeries, buildHeroCandleOptions } from '$lib/utils/apexCandlestick';

	interface Props {
		height?: number;
		days?: number;
		animationDelay?: number;
	}

	let { height = 400, days = 60, animationDelay = 0.5 }: Props = $props();

	let chartContainer: HTMLElement | undefined = $state();
	let visible = $state(false);

	$effect(() => {
		if (!chartContainer) return;
		const node = chartContainer;
		const opts = untrack(() => ({ height, days, animationDelay }));

		let cancelled = false;
		let chart: { render(): Promise<unknown>; destroy(): void } | null = null;
		let fadeTimer: ReturnType<typeof setTimeout> | undefined;

		void (async () => {
			const ApexCharts = (await import('apexcharts')).default;
			if (cancelled) return;

			const raw = generateTrendData(opts.days, 100, 'up', 0.8);
			const series = [{ name: 'Price', data: ohlcToApexSeries(raw) }];
			const options = buildHeroCandleOptions({ height: opts.height, series });

			const apx = new ApexCharts(node, options);
			chart = apx;
			await apx.render();

			if (cancelled) {
				apx.destroy();
				chart = null;
				return;
			}

			fadeTimer = setTimeout(() => {
				visible = true;
			}, opts.animationDelay * 1000);
		})();

		return () => {
			cancelled = true;
			clearTimeout(fadeTimer);
			chart?.destroy();
			chart = null;
		};
	});
</script>

<div
	bind:this={chartContainer}
	class={['hero-chart', { 'hero-chart--visible': visible }]}
	style:height="{height}px"
></div>

<style>
	.hero-chart {
		width: 100%;
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;
		opacity: 0;
		transition: opacity 1.5s ease-out;
	}

	.hero-chart--visible {
		opacity: 1;
	}

	:global(.hero-chart .apexcharts-canvas),
	:global(.hero-chart .apexcharts-svg) {
		opacity: 0.4 !important;
	}
</style>
