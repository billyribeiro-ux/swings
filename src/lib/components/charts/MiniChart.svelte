<script lang="ts">
	import { onMount } from 'svelte';
	import { createChart, ColorType, CandlestickSeries } from 'lightweight-charts';
	import { miniChartTheme } from '$lib/utils/chartThemes';
	import { generateTrendData } from '$lib/utils/chartData';

	interface Props {
		ticker?: string;
		trend?: 'up' | 'down' | 'sideways';
		height?: number;
		days?: number;
	}

	let { ticker = 'STOCK', trend = 'up', height = 80, days = 14 }: Props = $props();

	let chartContainer: HTMLElement | undefined = $state();
	let chart: ReturnType<typeof createChart> | null = null;

	onMount(() => {
		if (!chartContainer) return;

		// Create chart with compact theme
		chart = createChart(chartContainer, {
			layout: {
				background: { type: ColorType.Solid, color: 'transparent' },
				textColor: '#8b95a8',
				fontSize: 10,
				fontFamily: "'Inter', sans-serif"
			},
			grid: {
				vertLines: { visible: false },
				horzLines: { visible: false }
			},
			crosshair: { mode: 0 },
			rightPriceScale: { visible: false },
			timeScale: { visible: false },
			handleScroll: false,
			handleScale: false,
			autoSize: true
		});

		// Add candlestick series
		const candleSeries = chart.addSeries(CandlestickSeries, {
			upColor: miniChartTheme.candlestick.upColor,
			downColor: miniChartTheme.candlestick.downColor,
			borderUpColor: miniChartTheme.candlestick.borderUpColor,
			borderDownColor: miniChartTheme.candlestick.borderDownColor,
			wickUpColor: miniChartTheme.candlestick.wickUpColor,
			wickDownColor: miniChartTheme.candlestick.wickDownColor
		});

		// Generate and set data
		const data = generateTrendData(days, 100, trend, 0.6);
		candleSeries.setData(data);

		// Fit content
		chart.timeScale().fitContent();

		return () => {
			if (chart) {
				chart.remove();
				chart = null;
			}
		};
	});
</script>

<div bind:this={chartContainer} class="mini-chart" style="height: {height}px;"></div>

<style>
	.mini-chart {
		width: 100%;
		min-width: 120px;
		border-radius: var(--radius-md);
		overflow: hidden;
	}
</style>
