<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';

	interface Props {
		data: { cohort: string; retention: number[] }[];
		width?: number;
		height?: number;
	}

	let { data, width = 800, height = 400 }: Props = $props();

	let container = $state<HTMLDivElement | undefined>();
	let tooltipEl = $state<HTMLDivElement | undefined>();
	let mounted = $state(false);

	onMount(() => {
		mounted = true;
	});

	$effect(() => {
		if (!container || !mounted || !data || data.length === 0) return;

		d3.select(container).select('svg').remove();

		const maxMonths = d3.max(data, (d) => d.retention.length) ?? 0;
		if (maxMonths === 0) return;

		const margin = { top: 40, right: 30, bottom: 20, left: 80 };
		const w = container.clientWidth || width;
		const cellSize = Math.min(
			(w - margin.left - margin.right) / maxMonths,
			(height - margin.top - margin.bottom) / data.length,
			50
		);
		const innerW = cellSize * maxMonths;
		const innerH = cellSize * data.length;
		const totalW = innerW + margin.left + margin.right;
		const totalH = innerH + margin.top + margin.bottom;

		const svg = d3
			.select(container)
			.append('svg')
			.attr('width', totalW)
			.attr('height', totalH)
			.attr('viewBox', `0 0 ${totalW} ${totalH}`);

		const g = svg.append('g').attr('transform', `translate(${margin.left},${margin.top})`);

		// Color scale: dark navy (0%) -> bright teal (100%)
		const colorScale = d3
			.scaleLinear<string>()
			.domain([0, 50, 100])
			.range(['#132b50', '#0a6e76', '#15c5d1'])
			.clamp(true);

		// Column headers
		for (let col = 0; col < maxMonths; col++) {
			g.append('text')
				.attr('x', col * cellSize + cellSize / 2)
				.attr('y', -10)
				.attr('text-anchor', 'middle')
				.attr('fill', 'rgba(255,255,255,0.5)')
				.style('font-size', '10px')
				.style('font-weight', '500')
				.text(col === 0 ? 'M0' : `M${col}`);
		}

		// Row labels
		data.forEach((row, rowIdx) => {
			g.append('text')
				.attr('x', -10)
				.attr('y', rowIdx * cellSize + cellSize / 2 + 4)
				.attr('text-anchor', 'end')
				.attr('fill', 'rgba(255,255,255,0.6)')
				.style('font-size', '11px')
				.style('font-weight', '500')
				.text(row.cohort);
		});

		// Cells
		data.forEach((row, rowIdx) => {
			row.retention.forEach((val, colIdx) => {
				const cellGroup = g.append('g');

				cellGroup
					.append('rect')
					.attr('x', colIdx * cellSize + 1)
					.attr('y', rowIdx * cellSize + 1)
					.attr('width', cellSize - 2)
					.attr('height', cellSize - 2)
					.attr('rx', 3)
					.attr('ry', 3)
					.attr('fill', colorScale(val))
					.attr('stroke', 'rgba(11,29,58,0.8)')
					.attr('stroke-width', 1)
					.attr('opacity', 0)
					.transition()
					.duration(400)
					.delay(rowIdx * 40 + colIdx * 30)
					.ease(d3.easeCubicOut)
					.attr('opacity', 1);

				// Percentage text inside cell
				if (cellSize >= 30) {
					cellGroup
						.append('text')
						.attr('x', colIdx * cellSize + cellSize / 2)
						.attr('y', rowIdx * cellSize + cellSize / 2 + 4)
						.attr('text-anchor', 'middle')
						.attr('fill', val > 40 ? '#ffffff' : 'rgba(255,255,255,0.6)')
						.style('font-size', cellSize > 40 ? '11px' : '9px')
						.style('font-weight', '600')
						.text(`${val}%`)
						.attr('opacity', 0)
						.transition()
						.duration(400)
						.delay(rowIdx * 40 + colIdx * 30 + 200)
						.attr('opacity', 1);
				}

				// Hover
				cellGroup
					.append('rect')
					.attr('x', colIdx * cellSize + 1)
					.attr('y', rowIdx * cellSize + 1)
					.attr('width', cellSize - 2)
					.attr('height', cellSize - 2)
					.attr('rx', 3)
					.attr('fill', 'transparent')
					.style('cursor', 'pointer')
					.on('mouseenter', function (event: MouseEvent) {
						d3.select(this).attr('fill', 'rgba(255,255,255,0.08)');
						if (tooltipEl) {
							const rect = container!.getBoundingClientRect();
							tooltipEl.style.opacity = '1';
							tooltipEl.style.left = `${colIdx * cellSize + cellSize / 2 + margin.left}px`;
							tooltipEl.style.top = `${rowIdx * cellSize + margin.top - 45}px`;
							tooltipEl.innerHTML = `
								<div class="ch-tooltip__cohort">${row.cohort} &mdash; Month ${colIdx}</div>
								<div class="ch-tooltip__value">${val}% retained</div>
							`;
						}
					})
					.on('mouseleave', function () {
						d3.select(this).attr('fill', 'transparent');
						if (tooltipEl) {
							tooltipEl.style.opacity = '0';
						}
					});
			});
		});

		// Legend
		const legendW = 200;
		const legendH = 10;
		const legendX = innerW - legendW;
		const legendY = -30;

		const legendDefs = svg.append('defs');
		const legendGrad = legendDefs
			.append('linearGradient')
			.attr('id', 'heatmap-legend-grad')
			.attr('x1', '0%')
			.attr('x2', '100%');

		legendGrad.append('stop').attr('offset', '0%').attr('stop-color', '#132b50');
		legendGrad.append('stop').attr('offset', '50%').attr('stop-color', '#0a6e76');
		legendGrad.append('stop').attr('offset', '100%').attr('stop-color', '#15c5d1');

		const legend = svg.append('g').attr('transform', `translate(${margin.left + legendX},${margin.top + legendY})`);

		legend
			.append('rect')
			.attr('width', legendW)
			.attr('height', legendH)
			.attr('rx', 3)
			.attr('fill', 'url(#heatmap-legend-grad)');

		legend
			.append('text')
			.attr('x', 0)
			.attr('y', -4)
			.attr('fill', 'rgba(255,255,255,0.4)')
			.style('font-size', '9px')
			.text('0%');

		legend
			.append('text')
			.attr('x', legendW)
			.attr('y', -4)
			.attr('text-anchor', 'end')
			.attr('fill', 'rgba(255,255,255,0.4)')
			.style('font-size', '9px')
			.text('100%');

		return () => {
			if (container) d3.select(container).select('svg').remove();
		};
	});
</script>

<div class="ch-wrapper">
	<div class="ch-chart" bind:this={container}></div>
	<div class="ch-tooltip" bind:this={tooltipEl}></div>
</div>

<style>
	.ch-wrapper {
		position: relative;
		width: 100%;
		overflow-x: auto;
	}

	.ch-chart {
		width: 100%;
		min-width: 400px;
	}

	.ch-chart :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}

	.ch-tooltip {
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
		white-space: nowrap;
	}

	.ch-tooltip :global(.ch-tooltip__cohort) {
		font-size: 0.7rem;
		color: rgba(255, 255, 255, 0.5);
		margin-bottom: 2px;
	}

	.ch-tooltip :global(.ch-tooltip__value) {
		font-size: 0.9rem;
		font-weight: 700;
		color: #15c5d1;
	}
</style>
