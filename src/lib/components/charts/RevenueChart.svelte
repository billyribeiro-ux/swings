<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';

	interface DataPoint {
		date: string;
		revenue_cents: number;
	}

	interface Props {
		data: DataPoint[];
		width?: number;
		height?: number;
	}

	let { data, width = 800, height = 360 }: Props = $props();

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

	function formatDate(dateStr: string): string {
		const d = new Date(dateStr);
		return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
	}

	$effect(() => {
		if (!container || !mounted || !data || data.length === 0) return;

		// Clear previous render
		d3.select(container).select('svg').remove();

		const margin = { top: 20, right: 30, bottom: 40, left: 70 };
		const w = container.clientWidth || width;
		const h = height;
		const innerW = w - margin.left - margin.right;
		const innerH = h - margin.top - margin.bottom;

		const svg = d3
			.select(container)
			.append('svg')
			.attr('width', w)
			.attr('height', h)
			.attr('viewBox', `0 0 ${w} ${h}`)
			.style('overflow', 'visible');

		const g = svg.append('g').attr('transform', `translate(${margin.left},${margin.top})`);

		// Defs for gradient
		const defs = svg.append('defs');
		const gradient = defs
			.append('linearGradient')
			.attr('id', 'revenue-area-gradient')
			.attr('x1', '0%')
			.attr('y1', '0%')
			.attr('x2', '0%')
			.attr('y2', '100%');

		gradient.append('stop').attr('offset', '0%').attr('stop-color', '#0fa4af').attr('stop-opacity', 0.5);
		gradient.append('stop').attr('offset', '100%').attr('stop-color', '#0fa4af').attr('stop-opacity', 0.02);

		// Clip path for animation
		const clipId = 'revenue-clip';
		defs
			.append('clipPath')
			.attr('id', clipId)
			.append('rect')
			.attr('x', 0)
			.attr('y', 0)
			.attr('height', innerH + margin.top)
			.attr('width', 0)
			.transition()
			.duration(1200)
			.ease(d3.easeCubicOut)
			.attr('width', innerW);

		// Scales
		const parseDate = (d: string) => new Date(d);
		const xScale = d3
			.scaleTime()
			.domain(d3.extent(data, (d) => parseDate(d.date)) as [Date, Date])
			.range([0, innerW]);

		const maxRevenue = d3.max(data, (d) => d.revenue_cents) ?? 0;
		const yScale = d3
			.scaleLinear()
			.domain([0, maxRevenue * 1.1])
			.nice()
			.range([innerH, 0]);

		// Grid lines
		const yTicks = yScale.ticks(5);
		g.selectAll('.grid-line')
			.data(yTicks)
			.enter()
			.append('line')
			.attr('class', 'grid-line')
			.attr('x1', 0)
			.attr('x2', innerW)
			.attr('y1', (d) => yScale(d))
			.attr('y2', (d) => yScale(d))
			.attr('stroke', 'rgba(255,255,255,0.06)')
			.attr('stroke-dasharray', '3,3');

		// Area
		const area = d3
			.area<DataPoint>()
			.x((d) => xScale(parseDate(d.date)))
			.y0(innerH)
			.y1((d) => yScale(d.revenue_cents))
			.curve(d3.curveMonotoneX);

		g.append('path')
			.datum(data)
			.attr('fill', 'url(#revenue-area-gradient)')
			.attr('d', area)
			.attr('clip-path', `url(#${clipId})`);

		// Line
		const line = d3
			.line<DataPoint>()
			.x((d) => xScale(parseDate(d.date)))
			.y((d) => yScale(d.revenue_cents))
			.curve(d3.curveMonotoneX);

		g.append('path')
			.datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#0fa4af')
			.attr('stroke-width', 2.5)
			.attr('d', line)
			.attr('clip-path', `url(#${clipId})`);

		// Glow line
		g.append('path')
			.datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#0fa4af')
			.attr('stroke-width', 6)
			.attr('stroke-opacity', 0.2)
			.attr('d', line)
			.attr('filter', 'blur(4px)')
			.attr('clip-path', `url(#${clipId})`);

		// Axes
		const xAxis = d3
			.axisBottom(xScale)
			.ticks(Math.min(data.length, 8))
			.tickFormat((d) => {
				const date = d as Date;
				return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
			})
			.tickSize(0)
			.tickPadding(12);

		g.append('g')
			.attr('transform', `translate(0,${innerH})`)
			.call(xAxis)
			.call((sel) => sel.select('.domain').remove())
			.selectAll('text')
			.attr('fill', 'rgba(255,255,255,0.45)')
			.style('font-size', '11px');

		const yAxis = d3
			.axisLeft(yScale)
			.ticks(5)
			.tickFormat((d) => formatDollars(d as number))
			.tickSize(0)
			.tickPadding(12);

		g.append('g')
			.call(yAxis)
			.call((sel) => sel.select('.domain').remove())
			.selectAll('text')
			.attr('fill', 'rgba(255,255,255,0.45)')
			.style('font-size', '11px');

		// Tooltip interaction overlay
		const bisector = d3.bisector<DataPoint, Date>((d) => parseDate(d.date)).left;

		const focus = g.append('g').style('display', 'none');

		focus
			.append('line')
			.attr('class', 'focus-line')
			.attr('y1', 0)
			.attr('y2', innerH)
			.attr('stroke', 'rgba(255,255,255,0.15)')
			.attr('stroke-width', 1);

		focus
			.append('circle')
			.attr('r', 5)
			.attr('fill', '#0fa4af')
			.attr('stroke', '#0b1d3a')
			.attr('stroke-width', 2);

		// Glow circle
		focus
			.append('circle')
			.attr('r', 10)
			.attr('fill', 'rgba(15,164,175,0.25)')
			.attr('stroke', 'none');

		const overlay = g
			.append('rect')
			.attr('width', innerW)
			.attr('height', innerH)
			.attr('fill', 'transparent')
			.style('cursor', 'crosshair');

		overlay.on('mousemove', function (event: MouseEvent) {
			const [mx] = d3.pointer(event);
			const x0 = xScale.invert(mx);
			const i = bisector(data, x0, 1);
			const d0 = data[i - 1];
			const d1 = data[i];
			if (!d0) return;
			const d = d1 && x0.getTime() - parseDate(d0.date).getTime() > parseDate(d1.date).getTime() - x0.getTime() ? d1 : d0;

			const px = xScale(parseDate(d.date));
			const py = yScale(d.revenue_cents);

			focus.style('display', null);
			focus.select('line.focus-line').attr('x1', px).attr('x2', px);
			focus.selectAll('circle').attr('cx', px).attr('cy', py);

			if (tooltipEl) {
				tooltipEl.style.opacity = '1';
				tooltipEl.style.left = `${px + margin.left}px`;
				tooltipEl.style.top = `${py + margin.top - 50}px`;
				tooltipEl.innerHTML = `
					<div class="rc-tooltip__date">${formatDate(d.date)}</div>
					<div class="rc-tooltip__value">${formatDollars(d.revenue_cents)}</div>
				`;
			}
		});

		overlay.on('mouseleave', function () {
			focus.style('display', 'none');
			if (tooltipEl) {
				tooltipEl.style.opacity = '0';
			}
		});

		return () => {
			if (container) d3.select(container).select('svg').remove();
		};
	});
</script>

<div class="rc-wrapper">
	<div class="rc-chart" bind:this={container}></div>
	<div class="rc-tooltip" bind:this={tooltipEl}></div>
</div>

<style>
	.rc-wrapper {
		position: relative;
		width: 100%;
	}

	.rc-chart {
		width: 100%;
		overflow: hidden;
	}

	.rc-chart :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}

	.rc-tooltip {
		position: absolute;
		pointer-events: none;
		opacity: 0;
		background: rgba(11, 29, 58, 0.95);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: 8px;
		padding: 8px 12px;
		transform: translateX(-50%);
		transition: opacity 150ms ease;
		z-index: 10;
		backdrop-filter: blur(8px);
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
	}

	.rc-tooltip :global(.rc-tooltip__date) {
		font-size: 0.7rem;
		color: rgba(255, 255, 255, 0.5);
		margin-bottom: 2px;
	}

	.rc-tooltip :global(.rc-tooltip__value) {
		font-size: 0.95rem;
		font-weight: 700;
		color: #0fa4af;
	}
</style>
