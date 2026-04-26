<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';
	import type { AnalyticsTimeBucket } from '$lib/api/types';

	interface Props {
		data: AnalyticsTimeBucket[];
		width?: number;
		height?: number;
	}

	let { data, width = 800, height = 300 }: Props = $props();

	let container = $state<HTMLDivElement | undefined>();
	let mounted = $state(false);

	onMount(() => {
		mounted = true;
	});

	$effect(() => {
		if (!container || !mounted || !data?.length) return;

		d3.select(container).select('svg').remove();

		const margin = { top: 16, right: 56, bottom: 36, left: 52 };
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

		const parseDate = (d: string) => new Date(d + 'T12:00:00Z');
		const xScale = d3
			.scaleTime()
			.domain(d3.extent(data, (d) => parseDate(d.date)) as [Date, Date])
			.range([0, innerW]);

		const maxPvSess = Math.max(
			d3.max(data, (d) => d.page_views) ?? 0,
			d3.max(data, (d) => d.unique_sessions) ?? 0,
			1
		);
		const yLeft = d3
			.scaleLinear()
			.domain([0, maxPvSess * 1.08])
			.nice()
			.range([innerH, 0]);

		const maxImp = Math.max(d3.max(data, (d) => d.impressions) ?? 0, 1);
		const yRight = d3
			.scaleLinear()
			.domain([0, maxImp * 1.08])
			.nice()
			.range([innerH, 0]);

		const linePv = d3
			.line<AnalyticsTimeBucket>()
			.x((d) => xScale(parseDate(d.date)))
			.y((d) => yLeft(d.page_views))
			.curve(d3.curveMonotoneX);

		const lineSess = d3
			.line<AnalyticsTimeBucket>()
			.x((d) => xScale(parseDate(d.date)))
			.y((d) => yLeft(d.unique_sessions))
			.curve(d3.curveMonotoneX);

		const lineImp = d3
			.line<AnalyticsTimeBucket>()
			.x((d) => xScale(parseDate(d.date)))
			.y((d) => yRight(d.impressions))
			.curve(d3.curveMonotoneX);

		g.selectAll('.grid-line')
			.data(yLeft.ticks(5))
			.enter()
			.append('line')
			.attr('x1', 0)
			.attr('x2', innerW)
			.attr('y1', (d) => yLeft(d))
			.attr('y2', (d) => yLeft(d))
			.attr('stroke', 'rgba(255,255,255,0.06)')
			.attr('stroke-dasharray', '3,3');

		g.append('path')
			.datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#f59e0b')
			.attr('stroke-width', 1.5)
			.attr('stroke-dasharray', '4 3')
			.attr('opacity', 0.85)
			.attr('d', lineImp);

		g.append('path')
			.datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#6366f1')
			.attr('stroke-width', 2)
			.attr('d', lineSess);

		g.append('path')
			.datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#0fa4af')
			.attr('stroke-width', 2.5)
			.attr('d', linePv);

		g.append('g')
			.attr('transform', `translate(0,${innerH})`)
			.call(d3.axisBottom(xScale).ticks(6))
			.selectAll('text')
			.attr('fill', '#64748b')
			.style('font-size', '10px');

		g.append('g')
			.call(d3.axisLeft(yLeft).ticks(5).tickFormat(d3.format('~s')))
			.selectAll('text')
			.attr('fill', '#64748b')
			.style('font-size', '10px');

		g.append('g')
			.attr('transform', `translate(${innerW},0)`)
			.call(d3.axisRight(yRight).ticks(5).tickFormat(d3.format('~s')))
			.selectAll('text')
			.attr('fill', '#f59e0b')
			.style('font-size', '10px');

		g.selectAll('.domain, .tick line').attr('stroke', 'rgba(255,255,255,0.12)');

		const legend = svg.append('g').attr('transform', `translate(${margin.left}, 4)`);
		const leg = [
			{ c: '#0fa4af', t: 'Page views' },
			{ c: '#6366f1', t: 'Sessions' },
			{ c: '#f59e0b', t: 'Impr.', dash: '4 3' }
		];
		leg.forEach((item, i) => {
			const gx = i * 108;
			if (item.dash) {
				legend
					.append('line')
					.attr('x1', gx - 4)
					.attr('x2', gx + 4)
					.attr('y1', 4)
					.attr('y2', 4)
					.attr('stroke', item.c)
					.attr('stroke-width', 2)
					.attr('stroke-dasharray', item.dash);
			} else {
				legend
					.append('circle')
					.attr('cx', gx)
					.attr('cy', 4)
					.attr('r', 4)
					.attr('fill', item.c);
			}
			legend
				.append('text')
				.attr('x', gx + 10)
				.attr('y', 7)
				.attr('fill', '#94a3b8')
				.style('font-size', '11px')
				.text(item.t);
		});
	});
</script>

<div bind:this={container} class="traffic-chart"></div>

<style>
	.traffic-chart {
		width: 100%;
		min-height: 300px;
	}
</style>
