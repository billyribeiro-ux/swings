<script lang="ts">
	import { onMount } from 'svelte';
	import { generateTrendData } from '$lib/utils/chartData';
	import { ohlcToApexSeries, buildHeroCandleOptions } from '$lib/utils/apexCandlestick';

	interface Props {
		height?: number;
		days?: number;
		animationDelay?: number;
	}

	let { height = 400, days = 60, animationDelay = 0.5 }: Props = $props();

	let chartContainer: HTMLElement | undefined = $state();

	onMount(() => {
		if (!chartContainer) return;

		let cancelled = false;
		let chart: { render(): Promise<unknown>; destroy(): void } | null = null;

		void (async () => {
			const ApexCharts = (await import('apexcharts')).default;
			if (cancelled || !chartContainer) return;

			const raw = generateTrendData(days, 100, 'up', 0.8);
			const series = [{ name: 'Price', data: ohlcToApexSeries(raw) }];
			const options = buildHeroCandleOptions({ height, series });

			const apx = new ApexCharts(chartContainer, options);
			chart = apx;
			await apx.render();

			if (cancelled) {
				apx.destroy();
				chart = null;
				return;
			}

			setTimeout(() => {
				if (chartContainer) chartContainer.style.opacity = '1';
			}, animationDelay * 1000);
		})();

		return () => {
			cancelled = true;
			chart?.destroy();
			chart = null;
		};
	});
</script>

<div
	bind:this={chartContainer}
	class="hero-chart"
	style="height: {height}px; opacity: 0; transition: opacity 1.5s ease-out;"
></div>

<style>
	.hero-chart {
		width: 100%;
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;
	}

	:global(.hero-chart .apexcharts-canvas),
	:global(.hero-chart .apexcharts-svg) {
		opacity: 0.4 !important;
	}
</style>
