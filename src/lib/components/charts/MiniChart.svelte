<script lang="ts">
	import { onMount } from 'svelte';
	import { generateTrendData } from '$lib/utils/chartData';
	import { ohlcToApexSeries, buildMiniCandleOptions } from '$lib/utils/apexCandlestick';

	interface Props {
		ticker?: string;
		trend?: 'up' | 'down' | 'sideways';
		height?: number;
		days?: number;
		/** Richer grid for marketing cards */
		showcase?: boolean;
	}

	let {
		ticker = 'STOCK',
		trend = 'up',
		height = 80,
		days = 14,
		showcase = false
	}: Props = $props();

	let chartContainer: HTMLElement | undefined = $state();

	onMount(() => {
		if (!chartContainer) return;

		let cancelled = false;
		let chart: { render(): Promise<unknown>; destroy(): void } | null = null;

		void (async () => {
			const ApexCharts = (await import('apexcharts')).default;
			if (cancelled || !chartContainer) return;

			const raw = generateTrendData(days, 100, trend, showcase ? 0.75 : 0.6);
			const series = [{ name: 'Price', data: ohlcToApexSeries(raw) }];
			const options = buildMiniCandleOptions({ height, series, showcase });

			const apx = new ApexCharts(chartContainer, options);
			chart = apx;
			await apx.render();

			if (cancelled) {
				apx.destroy();
				chart = null;
			}
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
	class="mini-chart"
	class:mini-chart--showcase={showcase}
	style="height: {height}px;"
	role="img"
	aria-label="Price chart for {ticker}, last {days} sessions"
></div>

<style>
	.mini-chart {
		width: 100%;
		min-width: 120px;
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.mini-chart--showcase {
		border-radius: var(--radius-lg);
	}

	.mini-chart--showcase :global(.apexcharts-canvas) {
		filter: drop-shadow(0 1px 12px rgba(15, 164, 175, 0.12));
	}
</style>
