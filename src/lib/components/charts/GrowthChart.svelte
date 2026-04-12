<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';

	interface DataPoint {
		month: string;
		growth_percent: number;
		revenue_cents: number;
	}

	interface Props {
		data: DataPoint[];
		width?: number;
		height?: number;
	}

	let { data, width = 800, height = 340 }: Props = $props();

	let container = $state<HTMLDivElement | undefined>();
	let tooltipEl = $state<HTMLDivElement | undefined>();
	let mounted = $state(false);

	onMount(() => {
		mounted = true;
	});

	function formatDollars(cents: number): string {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: 'USD',
			minimumFractionDigits: 0,
			maximumFractionDigits: 0
		}).format(cents / 100);
	}

	$effect(() => {
		if (!container || !mounted || !data || data.length === 0) return;

		d3.select(container).select('svg').remove();

		const margin = { top: 30, right: 20, bottom: 40, left: 60 };
		const w = container.clientWidth || width;
		const h = height;
		const innerW = w - margin.left - margin.right;
		const innerH = h - margin.top - margin.bottom;

		const svg = d3
			.select(container)
			.append('svg')
			.attr('width', w)
			.attr('height', h)
			.attr('viewBox', `0 0 ${w} ${h}`);

		const g = svg.append('g').attr('transform', `translate(${margin.left},${margin.top})`);

		// Defs for rounded bar tops
		const defs = svg.append('defs');

		// Filters for glow
		const filter = defs.append('filter').attr('id', 'growth-bar-glow');
		filter.append('feGaussianBlur').attr('stdDeviation', '3').attr('result', 'blur');
		filter
			.append('feMerge')
			.selectAll('feMergeNode')
			.data(['blur', 'SourceGraphic'])
			.enter()
			.append('feMergeNode')
			.attr('in', (d) => d);

		// Scales
		const xScale = d3
			.scaleBand()
			.domain(data.map((d) => d.month))
			.range([0, innerW])
			.padding(0.35);

		const maxAbs = Math.max(
			d3.max(data, (d) => Math.abs(d.growth_percent)) ?? 10,
			5
		);
		const yScale = d3
			.scaleLinear()
			.domain([-maxAbs * 1.2, maxAbs * 1.2])
			.nice()
			.range([innerH, 0]);

		// Grid lines
		const yTicks = yScale.ticks(6);
		g.selectAll('.grid-line')
			.data(yTicks)
			.enter()
			.append('line')
			.attr('x1', 0)
			.attr('x2', innerW)
			.attr('y1', (d) => yScale(d))
			.attr('y2', (d) => yScale(d))
			.attr('stroke', 'rgba(255,255,255,0.05)')
			.attr('stroke-dasharray', '3,3');

		// Zero line
		g.append('line')
			.attr('x1', 0)
			.attr('x2', innerW)
			.attr('y1', yScale(0))
			.attr('y2', yScale(0))
			.attr('stroke', 'rgba(255,255,255,0.15)')
			.attr('stroke-width', 1);

		// Bars
		const barWidth = xScale.bandwidth();
		const radius = Math.min(barWidth / 2, 6);

		g.selectAll('.growth-bar')
			.data(data)
			.enter()
			.append('rect')
			.attr('class', 'growth-bar')
			.attr('x', (d) => xScale(d.month)!)
			.attr('width', barWidth)
			.attr('rx', radius)
			.attr('ry', radius)
			.attr('fill', (d) => (d.growth_percent >= 0 ? '#0fa4af' : '#e04848'))
			.attr('fill-opacity', 0.85)
			.attr('y', yScale(0))
			.attr('height', 0)
			.transition()
			.duration(800)
			.delay((_, i) => i * 60)
			.ease(d3.easeCubicOut)
			.attr('y', (d) => (d.growth_percent >= 0 ? yScale(d.growth_percent) : yScale(0)))
			.attr('height', (d) => Math.abs(yScale(d.growth_percent) - yScale(0)));

		// Bar labels
		g.selectAll('.bar-label')
			.data(data)
			.enter()
			.append('text')
			.attr('class', 'bar-label')
			.attr('x', (d) => xScale(d.month)! + barWidth / 2)
			.attr('text-anchor', 'middle')
			.attr('fill', (d) => (d.growth_percent >= 0 ? '#15c5d1' : '#f87171'))
			.style('font-size', '11px')
			.style('font-weight', '600')
			.text((d) => `${d.growth_percent > 0 ? '+' : ''}${d.growth_percent.toFixed(1)}%`)
			.attr('y', yScale(0))
			.attr('opacity', 0)
			.transition()
			.duration(800)
			.delay((_, i) => i * 60 + 400)
			.ease(d3.easeCubicOut)
			.attr('y', (d) => (d.growth_percent >= 0 ? yScale(d.growth_percent) - 8 : yScale(d.growth_percent) + 16))
			.attr('opacity', 1);

		// X axis
		const xAxis = d3.axisBottom(xScale).tickSize(0).tickPadding(12);
		g.append('g')
			.attr('transform', `translate(0,${innerH})`)
			.call(xAxis)
			.call((sel) => sel.select('.domain').remove())
			.selectAll('text')
			.attr('fill', 'rgba(255,255,255,0.45)')
			.style('font-size', '11px');

		// Y axis
		const yAxis = d3
			.axisLeft(yScale)
			.ticks(6)
			.tickFormat((d) => `${d}%`)
			.tickSize(0)
			.tickPadding(10);

		g.append('g')
			.call(yAxis)
			.call((sel) => sel.select('.domain').remove())
			.selectAll('text')
			.attr('fill', 'rgba(255,255,255,0.45)')
			.style('font-size', '11px');

		// Tooltip interaction
		const hoverBars = g
			.selectAll('.hover-bar')
			.data(data)
			.enter()
			.append('rect')
			.attr('x', (d) => xScale(d.month)!)
			.attr('y', 0)
			.attr('width', barWidth)
			.attr('height', innerH)
			.attr('fill', 'transparent')
			.style('cursor', 'pointer');

		hoverBars
			.on('mouseenter', function (event: MouseEvent, d: DataPoint) {
				d3.select(this).attr('fill', 'rgba(255,255,255,0.03)');
				if (tooltipEl) {
					const xPos = xScale(d.month)! + barWidth / 2 + margin.left;
					const yPos = d.growth_percent >= 0 ? yScale(d.growth_percent) + margin.top - 60 : yScale(0) + margin.top - 60;
					tooltipEl.style.opacity = '1';
					tooltipEl.style.left = `${xPos}px`;
					tooltipEl.style.top = `${yPos}px`;
					tooltipEl.innerHTML = `
						<div class="gc-tooltip__month">${d.month}</div>
						<div class="gc-tooltip__growth" style="color:${d.growth_percent >= 0 ? '#0fa4af' : '#e04848'}">${d.growth_percent > 0 ? '+' : ''}${d.growth_percent.toFixed(1)}%</div>
						<div class="gc-tooltip__rev">${formatDollars(d.revenue_cents)}</div>
					`;
				}
			})
			.on('mouseleave', function () {
				d3.select(this).attr('fill', 'transparent');
				if (tooltipEl) {
					tooltipEl.style.opacity = '0';
				}
			});

		return () => {
			d3.select(container).select('svg').remove();
		};
	});
</script>

<div class="gc-wrapper">
	<div class="gc-chart" bind:this={container}></div>
	<div class="gc-tooltip" bind:this={tooltipEl}></div>
</div>

<style>
	.gc-wrapper {
		position: relative;
		width: 100%;
	}

	.gc-chart {
		width: 100%;
		overflow: hidden;
	}

	.gc-chart :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}

	.gc-tooltip {
		position: absolute;
		pointer-events: none;
		opacity: 0;
		background: rgba(11, 29, 58, 0.95);
		border: 1px solid rgba(15, 164, 175, 0.25);
		border-radius: 8px;
		padding: 8px 12px;
		transform: translateX(-50%);
		transition: opacity 150ms ease;
		z-index: 10;
		backdrop-filter: blur(8px);
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
		white-space: nowrap;
	}

	.gc-tooltip :global(.gc-tooltip__month) {
		font-size: 0.7rem;
		color: rgba(255, 255, 255, 0.5);
		margin-bottom: 2px;
	}

	.gc-tooltip :global(.gc-tooltip__growth) {
		font-size: 0.95rem;
		font-weight: 700;
	}

	.gc-tooltip :global(.gc-tooltip__rev) {
		font-size: 0.75rem;
		color: rgba(255, 255, 255, 0.6);
		margin-top: 1px;
	}
</style>
