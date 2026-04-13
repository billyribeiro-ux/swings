<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';

	interface FunnelStage {
		stage: string;
		count: number;
		label: string;
	}

	interface Props {
		data: FunnelStage[];
		width?: number;
		height?: number;
	}

	let { data, width = 800, height = 280 }: Props = $props();

	let container = $state<HTMLDivElement | undefined>();
	let mounted = $state(false);

	onMount(() => {
		mounted = true;
	});

	function formatNumber(n: number): string {
		if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
		if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
		return n.toLocaleString();
	}

	$effect(() => {
		if (!container || !mounted || !data || data.length === 0) return;

		d3.select(container).select('svg').remove();

		const margin = { top: 20, right: 40, bottom: 20, left: 40 };
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

		const defs = svg.append('defs');

		// Gradient for each stage
		data.forEach((_, i) => {
			const gradient = defs
				.append('linearGradient')
				.attr('id', `funnel-grad-${i}`)
				.attr('x1', '0%')
				.attr('y1', '0%')
				.attr('x2', '100%')
				.attr('y2', '0%');

			const t = i / Math.max(data.length - 1, 1);
			const r = Math.round(15 + t * 0);
			const gVal = Math.round(164 - t * 80);
			const b = Math.round(175 - t * 40);
			const colorStart = `rgb(${r}, ${gVal}, ${b})`;
			const colorEnd = `rgb(${Math.max(r - 20, 0)}, ${Math.max(gVal - 30, 80)}, ${Math.max(b - 20, 100)})`;

			gradient.append('stop').attr('offset', '0%').attr('stop-color', colorStart).attr('stop-opacity', 0.9);
			gradient.append('stop').attr('offset', '100%').attr('stop-color', colorEnd).attr('stop-opacity', 0.7);
		});

		const maxCount = d3.max(data, (d) => d.count) ?? 1;
		const stageWidth = innerW / data.length;
		const maxBarHeight = innerH * 0.8;
		const centerY = innerH / 2;

		// Draw funnel trapezoids
		data.forEach((d, i) => {
			const barH = (d.count / maxCount) * maxBarHeight;
			const nextBarH = i < data.length - 1 ? (data[i + 1].count / maxCount) * maxBarHeight : barH * 0.7;

			const x1 = i * stageWidth;
			const x2 = (i + 1) * stageWidth;
			const gap = 3;

			const topLeft = centerY - barH / 2;
			const bottomLeft = centerY + barH / 2;
			const topRight = centerY - nextBarH / 2;
			const bottomRight = centerY + nextBarH / 2;

			// Trapezoid path
			const path = `
				M ${x1 + gap} ${topLeft}
				L ${x2 - gap} ${topRight}
				L ${x2 - gap} ${bottomRight}
				L ${x1 + gap} ${bottomLeft}
				Z
			`;

			g.append('path')
				.attr('d', path)
				.attr('fill', `url(#funnel-grad-${i})`)
				.attr('stroke', 'rgba(15,164,175,0.3)')
				.attr('stroke-width', 1)
				.attr('opacity', 0)
				.transition()
				.duration(600)
				.delay(i * 150)
				.ease(d3.easeCubicOut)
				.attr('opacity', 1);

			// Stage label
			const labelX = x1 + stageWidth / 2;

			g.append('text')
				.attr('x', labelX)
				.attr('y', centerY - barH / 2 - 22)
				.attr('text-anchor', 'middle')
				.attr('fill', 'rgba(255,255,255,0.85)')
				.style('font-size', '12px')
				.style('font-weight', '600')
				.text(d.label)
				.attr('opacity', 0)
				.transition()
				.duration(600)
				.delay(i * 150 + 200)
				.attr('opacity', 1);

			// Count
			g.append('text')
				.attr('x', labelX)
				.attr('y', centerY + 5)
				.attr('text-anchor', 'middle')
				.attr('fill', '#ffffff')
				.style('font-size', '16px')
				.style('font-weight', '700')
				.text(formatNumber(d.count))
				.attr('opacity', 0)
				.transition()
				.duration(600)
				.delay(i * 150 + 300)
				.attr('opacity', 1);

			// Conversion percentage between stages
			if (i < data.length - 1) {
				const convRate = ((data[i + 1].count / d.count) * 100).toFixed(1);
				const arrowX = x2;

				g.append('text')
					.attr('x', arrowX)
					.attr('y', centerY + barH / 2 + 20)
					.attr('text-anchor', 'middle')
					.attr('fill', '#0fa4af')
					.style('font-size', '11px')
					.style('font-weight', '600')
					.text(`${convRate}%`)
					.attr('opacity', 0)
					.transition()
					.duration(600)
					.delay(i * 150 + 400)
					.attr('opacity', 1);

				// Small arrow
				g.append('text')
					.attr('x', arrowX)
					.attr('y', centerY + barH / 2 + 34)
					.attr('text-anchor', 'middle')
					.attr('fill', 'rgba(15,164,175,0.5)')
					.style('font-size', '10px')
					.text('\u2192')
					.attr('opacity', 0)
					.transition()
					.duration(600)
					.delay(i * 150 + 450)
					.attr('opacity', 1);
			}
		});

		return () => {
			if (container) d3.select(container).select('svg').remove();
		};
	});
</script>

<div class="fc-wrapper">
	<div class="fc-chart" bind:this={container}></div>
</div>

<style>
	.fc-wrapper {
		position: relative;
		width: 100%;
	}

	.fc-chart {
		width: 100%;
		overflow: hidden;
	}

	.fc-chart :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}
</style>
