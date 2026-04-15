<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteDate } from 'svelte/reactivity';
	import { api } from '$lib/api/client';
	import type { AnalyticsSummary, AdminRevenueResponse } from '$lib/api/types';
	import RevenueChart from '$lib/components/charts/RevenueChart.svelte';
	import TrafficChart from '$lib/components/charts/TrafficChart.svelte';

	const RANGES: Record<string, string> = {
		'7d': '7 Days',
		'30d': '30 Days',
		'90d': '90 Days',
		ytd: 'Year to Date'
	};

	let range = $state('30d');
	let loading = $state(true);
	let error = $state<string | null>(null);
	let summary = $state<AnalyticsSummary | null>(null);
	let revenueData = $state<{ date: string; revenue_cents: number }[]>([]);

	function getRangeDates(r: string): { from: string; to: string } {
		const to = new SvelteDate();
		const from = new SvelteDate(to.getTime());
		if (r === '7d') from.setDate(to.getDate() - 7);
		else if (r === '30d') from.setDate(to.getDate() - 30);
		else if (r === '90d') from.setDate(to.getDate() - 90);
		else from.setMonth(0, 1);
		const fmt = (d: Date) => d.toISOString().slice(0, 10);
		return { from: fmt(from), to: fmt(to) };
	}

	function formatCents(cents: number): string {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: 'USD',
			minimumFractionDigits: 0,
			maximumFractionDigits: 0
		}).format(cents / 100);
	}

	function formatNumber(n: number): string {
		return new Intl.NumberFormat('en-US').format(n);
	}

	function formatPct(x: number): string {
		return `${(x * 100).toFixed(1)}%`;
	}

	function formatSaleTime(iso: string): string {
		const d = new Date(iso);
		return d.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
	}

	const ctrByCta = $derived.by(() => {
		if (!summary?.ctr_series?.length) return [];
		const acc: Record<string, { impressions: number; clicks: number }> = {};
		for (const row of summary.ctr_series) {
			const cur = acc[row.cta_id] ?? { impressions: 0, clicks: 0 };
			cur.impressions += row.impressions;
			cur.clicks += row.clicks;
			acc[row.cta_id] = cur;
		}
		return Object.entries(acc)
			.map(([cta_id, v]) => ({
				cta_id,
				impressions: v.impressions,
				clicks: v.clicks,
				ctr: v.impressions > 0 ? v.clicks / v.impressions : 0
			}))
			.sort((a, b) => b.impressions - a.impressions);
	});

	const kpiCards = $derived.by(() => {
		if (!summary) return [];
		return [
			{
				label: 'Page views',
				value: formatNumber(summary.total_page_views),
				hint: 'In selected range',
				accent: '#0fa4af'
			},
			{
				label: 'Sessions',
				value: formatNumber(summary.total_sessions),
				hint: 'Distinct sessions (page views)',
				accent: '#6366f1'
			},
			{
				label: 'Bounce rate',
				value: formatPct(summary.bounce_rate),
				hint: 'Sessions with exactly 1 page view',
				accent: '#f59e0b'
			},
			{
				label: 'CTA impressions',
				value: formatNumber(summary.total_impressions),
				hint: 'Tracked elements',
				accent: '#a855f7'
			},
			{
				label: 'Est. MRR',
				value: formatCents(summary.mrr_cents),
				hint: 'From active subs × plan prices',
				accent: '#10b981'
			},
			{
				label: 'Est. ARR',
				value: formatCents(summary.arr_cents),
				hint: 'MRR × 12',
				accent: '#14b8a6'
			},
			{
				label: 'Period revenue',
				value: formatCents(summary.period_revenue_cents),
				hint: 'Sum of sales_events in range',
				accent: '#eab308'
			},
			{
				label: 'Active subscribers',
				value: formatNumber(summary.active_subscribers),
				hint: 'Stripe subscription status',
				accent: '#3b82f6'
			}
		];
	});

	async function load() {
		loading = true;
		error = null;
		const { from, to } = getRangeDates(range);
		const query = `from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`;

		try {
			const [s, rev] = await Promise.all([
				api.get<AnalyticsSummary>(`/api/admin/analytics/summary?${query}`),
				api.get<AdminRevenueResponse>(`/api/admin/analytics/revenue?${query}`)
			]);
			summary = s;
			revenueData = rev.data ?? [];
		} catch (e) {
			summary = null;
			revenueData = [];
			error = e instanceof Error ? e.message : 'Failed to load analytics';
		}
		loading = false;
	}

	onMount(() => {
		load();
	});
</script>

<svelte:head><title>Analytics Dashboard - Admin</title></svelte:head>

<div class="ap">
	<header class="ap-header">
		<div>
			<h1 class="ap-title">Analytics</h1>
			<p class="ap-sub">
				First-party traffic, CTA performance, and subscription estimates (MRR uses active plans ×
				<code>pricing_plans</code>).
			</p>
		</div>
		<select class="ap-select" bind:value={range} onchange={() => load()}>
			{#each Object.entries(RANGES) as [key, label] (key)}
				<option value={key}>{label}</option>
			{/each}
		</select>
	</header>

	{#if loading}
		<div class="ap-loading">Loading analytics…</div>
	{:else if error}
		<div class="ap-error" role="alert">{error}</div>
	{:else if summary}
		<section class="ap-kpis">
			{#each kpiCards as card (card.label)}
				<div class="ap-kpi" style="--kpi-accent: {card.accent}" title={card.hint}>
					<span class="ap-kpi-label">{card.label}</span>
					<span class="ap-kpi-value">{card.value}</span>
					<span class="ap-kpi-hint">{card.hint}</span>
				</div>
			{/each}
		</section>

		<div class="ap-charts">
			<section class="ap-card ap-card--wide">
				<h2 class="ap-card-title">Traffic</h2>
				{#if summary.time_series.length}
					<TrafficChart data={summary.time_series} />
				{:else}
					<p class="ap-empty">No traffic in this range yet.</p>
				{/if}
			</section>
		</div>

		<div class="ap-charts">
			<section class="ap-card">
				<h2 class="ap-card-title">Revenue (sales_events)</h2>
				{#if revenueData.length}
					<RevenueChart data={revenueData} />
				{:else}
					<p class="ap-empty">No recorded sales in this range. Events are written from Stripe flows.</p>
				{/if}
			</section>
		</div>

		<div class="ap-bottom">
			<section class="ap-card">
				<h2 class="ap-card-title">Top pages</h2>
				<div class="ap-table-wrap">
					<table class="ap-table">
						<thead>
							<tr>
								<th>Path</th>
								<th>Views</th>
								<th>Sessions</th>
							</tr>
						</thead>
						<tbody>
							{#each summary.top_pages as page (page.path)}
								<tr>
									<td class="ap-table-path">{page.path}</td>
									<td>{formatNumber(page.views)}</td>
									<td>{formatNumber(page.sessions)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</section>

			<section class="ap-card">
				<h2 class="ap-card-title">CTA performance</h2>
				{#if ctrByCta.length === 0}
					<p class="ap-empty">No CTA impressions in range. Instrument with <code>ctaImpression</code>.</p>
				{:else}
					<div class="ap-table-wrap">
						<table class="ap-table">
							<thead>
								<tr>
									<th>CTA id</th>
									<th>Impr.</th>
									<th>Clicks</th>
									<th>CTR</th>
								</tr>
							</thead>
							<tbody>
								{#each ctrByCta as row (row.cta_id)}
									<tr>
										<td class="ap-table-path">{row.cta_id}</td>
										<td>{formatNumber(row.impressions)}</td>
										<td>{formatNumber(row.clicks)}</td>
										<td>{formatPct(row.ctr)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</section>
		</div>

		<section class="ap-card ap-card--activity">
			<h2 class="ap-card-title">Recent sales activity</h2>
			{#if summary.recent_sales.length === 0}
				<p class="ap-empty">No sales_events in this date range.</p>
			{:else}
				<ul class="ap-activity">
					{#each summary.recent_sales as item (item.id)}
						<li class="ap-activity-item">
							<div class="ap-activity-dot"></div>
							<div class="ap-activity-body">
								<span class="ap-activity-action">
									{item.event_type.replace(/_/g, ' ')} — {formatCents(item.amount_cents)}
								</span>
								<span class="ap-activity-meta">
									{item.user_email} · {formatSaleTime(item.created_at)}
								</span>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}
</div>

<style>
	.ap {
		max-width: 76rem;
		padding: 0 0 3rem;
	}
	.ap-header {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}
	.ap-title {
		margin: 0 0 0.3rem;
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--color-white, #f8fafc);
	}
	.ap-sub {
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-grey-500, #64748b);
		max-width: 42rem;
		line-height: 1.5;
	}
	.ap-sub code {
		font-size: 0.8em;
		padding: 0.1em 0.35em;
		border-radius: 0.25rem;
		background: rgba(255, 255, 255, 0.06);
	}
	.ap-select {
		padding: 0.5rem 0.85rem;
		border-radius: 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		background: rgba(0, 0, 0, 0.35);
		color: var(--color-white, #f8fafc);
		font-family: inherit;
		font-size: 0.875rem;
		cursor: pointer;
		outline: none;
	}
	.ap-select:focus {
		border-color: #0fa4af;
	}
	.ap-loading,
	.ap-error {
		text-align: center;
		padding: 4rem 0;
		color: var(--color-grey-500, #64748b);
	}
	.ap-error {
		color: #f87171;
	}
	.ap-kpis {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	.ap-kpi {
		padding: 1.1rem 1.25rem;
		border-radius: 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.07);
		background: rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(12px);
		border-top: 2px solid var(--kpi-accent);
	}
	.ap-kpi-label {
		display: block;
		font-size: 0.7rem;
		color: var(--color-grey-500, #64748b);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin-bottom: 0.35rem;
	}
	.ap-kpi-value {
		font-size: 1.35rem;
		font-weight: 700;
		color: var(--color-white, #f8fafc);
		font-variant-numeric: tabular-nums;
	}
	.ap-kpi-hint {
		display: block;
		margin-top: 0.35rem;
		font-size: 0.7rem;
		color: rgba(148, 163, 184, 0.9);
		line-height: 1.3;
	}
	.ap-charts {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1.5rem;
		margin-bottom: 1.5rem;
	}
	.ap-card {
		padding: 1.5rem;
		border-radius: 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.07);
		background: rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(12px);
	}
	.ap-card--wide {
		grid-column: 1 / -1;
	}
	.ap-card--activity {
		margin-top: 0;
	}
	.ap-card-title {
		margin: 0 0 1rem;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white, #f8fafc);
	}
	.ap-empty {
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-grey-500, #64748b);
		line-height: 1.5;
	}
	.ap-empty code {
		font-size: 0.85em;
	}
	.ap-bottom {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5rem;
		margin-bottom: 1.5rem;
	}
	.ap-table-wrap {
		overflow-x: auto;
	}
	.ap-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}
	.ap-table th {
		text-align: left;
		padding: 0.6rem 0.75rem;
		color: var(--color-grey-500, #64748b);
		font-weight: 500;
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}
	.ap-table td {
		padding: 0.6rem 0.75rem;
		color: rgba(255, 255, 255, 0.75);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		font-variant-numeric: tabular-nums;
	}
	.ap-table-path {
		color: #0fa4af;
		font-weight: 500;
		word-break: break-all;
	}
	.ap-table tr:last-child td {
		border-bottom: none;
	}
	.ap-activity {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.ap-activity-item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
	}
	.ap-activity-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #0fa4af;
		margin-top: 0.35rem;
		flex-shrink: 0;
	}
	.ap-activity-body {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	.ap-activity-action {
		font-size: 0.85rem;
		color: var(--color-white, #f8fafc);
		font-weight: 500;
		text-transform: capitalize;
	}
	.ap-activity-meta {
		font-size: 0.75rem;
		color: var(--color-grey-500, #64748b);
	}
	@media (max-width: 1100px) {
		.ap-kpis {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (max-width: 700px) {
		.ap-kpis {
			grid-template-columns: 1fr;
		}
		.ap-bottom {
			grid-template-columns: 1fr;
		}
	}
</style>
