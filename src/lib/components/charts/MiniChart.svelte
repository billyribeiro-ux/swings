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
		/** Richer grid + margins for hero / marketing cards */
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
	let chart: ReturnType<typeof createChart> | null = null;

	onMount(() => {
		if (!chartContainer) return;

		const grid = showcase
			? {
					vertLines: { visible: false },
					horzLines: {
						color: 'rgba(255, 255, 255, 0.06)',
						style: 1,
						visible: true
					}
				}
			: {
					vertLines: { visible: false },
					horzLines: { visible: false }
				};

		chart = createChart(chartContainer, {
			layout: {
				background: { type: ColorType.Solid, color: 'transparent' },
				textColor: 'rgba(148, 163, 184, 0.75)',
				fontSize: showcase ? 11 : 10,
				fontFamily: "ui-sans-serif, system-ui, sans-serif"
			},
			grid,
			crosshair: { mode: 0 },
			rightPriceScale: { visible: false },
			timeScale: { visible: false },
			handleScroll: false,
			handleScale: false,
			autoSize: true
		});

		const candleSeries = chart.addSeries(CandlestickSeries, {
			upColor: miniChartTheme.candlestick.upColor,
			downColor: miniChartTheme.candlestick.downColor,
			borderUpColor: miniChartTheme.candlestick.borderUpColor,
			borderDownColor: miniChartTheme.candlestick.borderDownColor,
			wickUpColor: miniChartTheme.candlestick.wickUpColor,
			wickDownColor: miniChartTheme.candlestick.wickDownColor
		});

		const data = generateTrendData(days, 100, trend, showcase ? 0.75 : 0.6);
		candleSeries.setData(data);

		chart.timeScale().fitContent();

		return () => {
			if (chart) {
				chart.remove();
				chart = null;
			}
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

	.mini-chart--showcase :global(canvas) {
		filter: drop-shadow(0 1px 12px rgba(15, 164, 175, 0.12));
	}
</style>
