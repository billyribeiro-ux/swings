<script lang="ts">
	import { untrack } from 'svelte';
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

	$effect(() => {
		if (!chartContainer) return;
		const node = chartContainer;
		const opts = untrack(() => ({ trend, height, days, showcase }));

		let cancelled = false;
		let chart: { render(): Promise<unknown>; destroy(): void } | null = null;

		void (async () => {
			const ApexCharts = (await import('apexcharts')).default;
			if (cancelled) return;

			const raw = generateTrendData(opts.days, 100, opts.trend, opts.showcase ? 0.75 : 0.6);
			const series = [{ name: 'Price', data: ohlcToApexSeries(raw) }];
			const options = buildMiniCandleOptions({
				height: opts.height,
				series,
				showcase: opts.showcase
			});

			const apx = new ApexCharts(node, options);
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
	class={['mini-chart', { 'mini-chart--showcase': showcase }]}
	style:height="{height}px"
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
