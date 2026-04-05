<script lang="ts">
	import { onMount } from 'svelte';
	import { createChart, type CandlestickData, type IChartApi } from 'lightweight-charts';
	import { heroChartTheme } from '$lib/utils/chartThemes';
	import { generateTrendData } from '$lib/utils/chartData';
	
	interface Props {
		height?: number;
		days?: number;
		animationDelay?: number;
	}
	
	let { 
		height = 400,
		days = 60,
		animationDelay = 0.5
	}: Props = $props();
	
	let chartContainer: HTMLElement | undefined = $state();
	let chart: IChartApi | null = null;
	
	onMount(() => {
		if (!chartContainer) return;
		
		chart = createChart(chartContainer, {
			layout: {
				background: { type: 'solid', color: 'transparent' },
				textColor: 'rgba(139, 149, 168, 0.5)',
				fontSize: 10,
				fontFamily: "'Inter', sans-serif"
			},
			grid: {
				vertLines: {
					color: 'rgba(15, 164, 175, 0.06)',
					visible: true
				},
				horzLines: {
					color: 'rgba(15, 164, 175, 0.06)',
					visible: true
				}
			},
			crosshair: {
				mode: 0,
				vertLine: { visible: false },
				horzLine: { visible: false }
			},
			rightPriceScale: { visible: false },
			timeScale: { visible: false },
			handleScroll: false,
			handleScale: false,
			autoSize: true
		});
		
		const candleSeries = chart.addCandlestickSeries({
			upColor: heroChartTheme.candlestick.upColor,
			downColor: heroChartTheme.candlestick.downColor,
			borderUpColor: heroChartTheme.candlestick.borderUpColor,
			borderDownColor: heroChartTheme.candlestick.borderDownColor,
			wickUpColor: 'rgba(34, 181, 115, 0.3)',
			wickDownColor: 'rgba(224, 72, 72, 0.3)'
		});
		
		// Generate bullish data
		const data = generateTrendData(days, 100, 'up', 0.8);
		candleSeries.setData(data);
		
		chart.timeScale().fitContent();
		
		// Animate chart appearance
		setTimeout(() => {
			if (chartContainer) {
				chartContainer.style.opacity = '1';
			}
		}, animationDelay * 1000);
		
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
	class="hero-chart"
	style="height: {height}px; opacity: 0; transition: opacity 1.5s ease-out;"
>
</div>

<style>
	.hero-chart {
		width: 100%;
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;
	}
	
	:global(.hero-chart canvas) {
		opacity: 0.4 !important;
	}
</style>
