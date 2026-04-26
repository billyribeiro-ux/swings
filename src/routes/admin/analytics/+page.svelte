<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteDate } from 'svelte/reactivity';
	import { api } from '$lib/api/client';
	import type { AnalyticsSummary, AdminRevenueResponse } from '$lib/api/types';
	import RevenueChart from '$lib/components/charts/RevenueChart.svelte';
	import TrafficChart from '$lib/components/charts/TrafficChart.svelte';
	import PresentationChartIcon from 'phosphor-svelte/lib/PresentationChartIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import DownloadSimpleIcon from 'phosphor-svelte/lib/DownloadSimpleIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import ArrowBendUpRightIcon from 'phosphor-svelte/lib/ArrowBendUpRightIcon';
	import CursorClickIcon from 'phosphor-svelte/lib/CursorClickIcon';
	import CurrencyDollarIcon from 'phosphor-svelte/lib/CurrencyDollarIcon';
	import TrendUpIcon from 'phosphor-svelte/lib/TrendUpIcon';
	import ReceiptIcon from 'phosphor-svelte/lib/ReceiptIcon';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';
	import ChartLineIcon from 'phosphor-svelte/lib/ChartLineIcon';
	import ChartBarIcon from 'phosphor-svelte/lib/ChartBarIcon';
	import ListBulletsIcon from 'phosphor-svelte/lib/ListBulletsIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import ChatCircleDotsIcon from 'phosphor-svelte/lib/ChatCircleDotsIcon';

	const PRESETS = [
		{ key: '7d', label: '7d' },
		{ key: '30d', label: '30d' },
		{ key: '90d', label: '90d' },
		{ key: 'ytd', label: 'YTD' }
	] as const;

	let range = $state<string>('30d');
	let customFrom = $state<string>('');
	let customTo = $state<string>('');
	let loading = $state(true);
	let error = $state<string | null>(null);
	let summary = $state<AnalyticsSummary | null>(null);
	let revenueData = $state<{ date: string; revenue_cents: number }[]>([]);

	function getRangeDates(r: string): { from: string; to: string } {
		if (r === 'custom' && customFrom && customTo) {
			return { from: customFrom, to: customTo };
		}
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
				icon: EyeIcon
			},
			{
				label: 'Sessions',
				value: formatNumber(summary.total_sessions),
				hint: 'Distinct sessions',
				icon: UsersIcon
			},
			{
				label: 'Bounce rate',
				value: formatPct(summary.bounce_rate),
				hint: 'Single-pageview sessions',
				icon: ArrowBendUpRightIcon
			},
			{
				label: 'CTA impressions',
				value: formatNumber(summary.total_impressions),
				hint: 'Tracked elements',
				icon: CursorClickIcon
			},
			{
				label: 'Est. MRR',
				value: formatCents(summary.mrr_cents),
				hint: 'Active subs × plan price',
				icon: CurrencyDollarIcon
			},
			{
				label: 'Est. ARR',
				value: formatCents(summary.arr_cents),
				hint: 'MRR × 12',
				icon: TrendUpIcon
			},
			{
				label: 'Period revenue',
				value: formatCents(summary.period_revenue_cents),
				hint: 'Sum of sales_events',
				icon: ReceiptIcon
			},
			{
				label: 'Active subscribers',
				value: formatNumber(summary.active_subscribers),
				hint: 'Stripe subscription status',
				icon: UserCircleIcon
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

	function selectPreset(key: string) {
		range = key;
		void load();
	}

	function applyCustom() {
		if (!customFrom || !customTo) return;
		range = 'custom';
		void load();
	}

	function exportCsv() {
		if (!summary) return;
		const rows: string[][] = [['metric', 'value']];
		for (const c of kpiCards) rows.push([c.label, c.value]);
		const csv = rows.map((r) => r.map((v) => `"${String(v).replace(/"/g, '""')}"`).join(',')).join('\n');
		const blob = new Blob([csv], { type: 'text/csv' });
		const a = document.createElement('a');
		a.href = URL.createObjectURL(blob);
		a.download = `analytics-${new Date().toISOString().slice(0, 10)}.csv`;
		document.body.appendChild(a);
		a.click();
		a.remove();
		URL.revokeObjectURL(a.href);
	}

	const { from: rangeFrom, to: rangeTo } = $derived(getRangeDates(range));

	onMount(() => {
		load();
	});
</script>

<svelte:head><title>Analytics Dashboard - Admin</title></svelte:head>

<div class="ap">
	<header class="ap-header">
		<div class="ap-header__title-row">
			<PresentationChartIcon size={22} weight="duotone" />
			<div class="ap-header__copy">
				<span class="ap-eyebrow">Analytics</span>
				<h1 class="ap-title">Analytics</h1>
				<p class="ap-sub">
					First-party traffic, CTA performance, and subscription estimates.
				</p>
			</div>
		</div>
		<div class="ap-toolbar">
			<div class="ap-presets" role="tablist" aria-label="Date range">
				{#each PRESETS as p (p.key)}
					<button
						type="button"
						role="tab"
						aria-selected={range === p.key}
						class="ap-preset"
						class:ap-preset--active={range === p.key}
						onclick={() => selectPreset(p.key)}
					>
						{p.label}
					</button>
				{/each}
			</div>
			<div class="ap-range" aria-label="Custom date range">
				<FunnelIcon size={14} weight="bold" />
				<input
					id="ap-from"
					name="from"
					type="date"
					class="ap-input ap-input--date"
					bind:value={customFrom}
					onchange={applyCustom}
					aria-label="From date"
				/>
				<span class="ap-range__sep">→</span>
				<input
					id="ap-to"
					name="to"
					type="date"
					class="ap-input ap-input--date"
					bind:value={customTo}
					onchange={applyCustom}
					aria-label="To date"
				/>
			</div>
			<div class="ap-actions">
				<button type="button" class="ap-btn ap-btn--ghost" onclick={() => load()} aria-label="Refresh">
					<ArrowsClockwiseIcon size={14} weight="bold" />
					<span>Refresh</span>
				</button>
				<button type="button" class="ap-btn ap-btn--ghost" onclick={exportCsv}>
					<DownloadSimpleIcon size={14} weight="bold" />
					<span>Export</span>
				</button>
			</div>
		</div>
	</header>

	<p class="ap-rangeline">
		<span class="ap-rangeline__eyebrow">Range</span>
		<span class="ap-rangeline__value">{rangeFrom} → {rangeTo}</span>
	</p>

	{#if loading}
		<div class="ap-loading" role="status">
			<div class="ap-spinner" aria-hidden="true"></div>
			<span>Loading analytics…</span>
		</div>
	{:else if error}
		<div class="ap-error" role="alert">{error}</div>
	{:else if summary}
		<section class="ap-kpis" aria-label="Key metrics">
			{#each kpiCards as card (card.label)}
				<div class="ap-kpi" title={card.hint}>
					<div class="ap-kpi__top">
						<span class="ap-kpi__label">{card.label}</span>
						<span class="ap-kpi__icon">
							<card.icon size={16} weight="duotone" />
						</span>
					</div>
					<span class="ap-kpi__value">{card.value}</span>
					<span class="ap-kpi__hint">{card.hint}</span>
				</div>
			{/each}
		</section>

		<section class="ap-card ap-card--full">
			<header class="ap-card__head">
				<span class="ap-card__eyebrow">Traffic</span>
				<h2 class="ap-card__title">
					<ChartLineIcon size={16} weight="duotone" />
					Page views over time
				</h2>
			</header>
			{#if summary.time_series.length}
				<TrafficChart data={summary.time_series} />
			{:else}
				<div class="ap-empty">
					<ChartLineIcon size={48} weight="duotone" />
					<p class="ap-empty__title">No traffic in this range</p>
					<p class="ap-empty__sub">Once visitors land, page views will appear here.</p>
				</div>
			{/if}
		</section>

		<section class="ap-card ap-card--full">
			<header class="ap-card__head">
				<span class="ap-card__eyebrow">Revenue</span>
				<h2 class="ap-card__title">
					<ChartBarIcon size={16} weight="duotone" />
					Sales events
				</h2>
			</header>
			{#if revenueData.length}
				<RevenueChart data={revenueData} />
			{:else}
				<div class="ap-empty">
					<CurrencyDollarIcon size={48} weight="duotone" />
					<p class="ap-empty__title">No recorded sales</p>
					<p class="ap-empty__sub">Events are written from Stripe checkout flows.</p>
				</div>
			{/if}
		</section>

		<div class="ap-bottom">
			<section class="ap-card">
				<header class="ap-card__head">
					<span class="ap-card__eyebrow">Top pages</span>
					<h2 class="ap-card__title">
						<ListBulletsIcon size={16} weight="duotone" />
						Most-viewed paths
					</h2>
				</header>
				<div class="ap-table-wrap">
					<table class="ap-table">
						<thead>
							<tr>
								<th scope="col">Path</th>
								<th scope="col" class="ap-table__num-th">Views</th>
								<th scope="col" class="ap-table__num-th">Sessions</th>
							</tr>
						</thead>
						<tbody>
							{#each summary.top_pages as page (page.path)}
								<tr>
									<td class="ap-table__path">{page.path}</td>
									<td class="ap-table__num">{formatNumber(page.views)}</td>
									<td class="ap-table__num">{formatNumber(page.sessions)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</section>

			<section class="ap-card">
				<header class="ap-card__head">
					<span class="ap-card__eyebrow">CTA performance</span>
					<h2 class="ap-card__title">
						<CursorClickIcon size={16} weight="duotone" />
						Click-through rates
					</h2>
				</header>
				{#if ctrByCta.length === 0}
					<div class="ap-empty">
						<CursorClickIcon size={48} weight="duotone" />
						<p class="ap-empty__title">No CTA impressions</p>
						<p class="ap-empty__sub">Instrument with <code>ctaImpression</code>.</p>
					</div>
				{:else}
					<div class="ap-table-wrap">
						<table class="ap-table">
							<thead>
								<tr>
									<th scope="col">CTA id</th>
									<th scope="col" class="ap-table__num-th">Impr.</th>
									<th scope="col" class="ap-table__num-th">Clicks</th>
									<th scope="col" class="ap-table__num-th">CTR</th>
								</tr>
							</thead>
							<tbody>
								{#each ctrByCta as row (row.cta_id)}
									<tr>
										<td class="ap-table__path">{row.cta_id}</td>
										<td class="ap-table__num">{formatNumber(row.impressions)}</td>
										<td class="ap-table__num">{formatNumber(row.clicks)}</td>
										<td class="ap-table__num">{formatPct(row.ctr)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</section>
		</div>

		<section class="ap-card">
			<header class="ap-card__head">
				<span class="ap-card__eyebrow">Activity</span>
				<h2 class="ap-card__title">
					<LightningIcon size={16} weight="duotone" />
					Recent sales
				</h2>
			</header>
			{#if summary.recent_sales.length === 0}
				<div class="ap-empty">
					<ChatCircleDotsIcon size={48} weight="duotone" />
					<p class="ap-empty__title">No sales events</p>
					<p class="ap-empty__sub">Sales appear once Stripe webhooks fire.</p>
				</div>
			{:else}
				<ul class="ap-activity">
					{#each summary.recent_sales as item (item.id)}
						<li class="ap-activity__item">
							<div class="ap-activity__dot" aria-hidden="true"></div>
							<div class="ap-activity__body">
								<span class="ap-activity__action">
									{item.event_type.replace(/_/g, ' ')} — {formatCents(item.amount_cents)}
								</span>
								<span class="ap-activity__meta">
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
		max-width: 80rem;
		padding: 0 0 3rem;
	}
	.ap-header {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		margin-bottom: 1.25rem;
	}
	.ap-header__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.ap-header__copy {
		min-width: 0;
	}
	.ap-eyebrow {
		display: block;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		line-height: 1;
		margin-bottom: 0.25rem;
	}
	.ap-title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.2;
	}
	.ap-sub {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		font-weight: 400;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		hyphens: none;
	}
	.ap-toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
	}
	.ap-presets {
		display: inline-flex;
		padding: 0.25rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		gap: 0.15rem;
	}
	.ap-preset {
		min-height: 2.5rem;
		padding: 0 0.75rem;
		background: transparent;
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-grey-300);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background-color 150ms,
			color 150ms;
	}
	.ap-preset:hover {
		color: var(--color-white);
		background: rgba(255, 255, 255, 0.05);
	}
	.ap-preset--active {
		background: rgba(15, 164, 175, 0.18);
		color: var(--color-teal);
	}
	.ap-range {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.25rem 0.65rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-400);
	}
	.ap-range__sep {
		color: var(--color-grey-500);
		font-size: 0.75rem;
	}
	.ap-input {
		min-height: 2.5rem;
		padding: 0.35rem 0.5rem;
		background: transparent;
		border: 1px solid transparent;
		color: var(--color-white);
		font-size: 0.875rem;
		font-weight: 400;
		border-radius: var(--radius-md);
		font-family: inherit;
		color-scheme: dark;
		transition:
			border-color 150ms,
			box-shadow 150ms;
	}
	.ap-input:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}
	.ap-actions {
		display: inline-flex;
		gap: 0.4rem;
		margin-left: auto;
	}
	.ap-btn {
		min-height: 3rem;
		padding: 0 0.85rem;
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8125rem;
		font-weight: 600;
		border-radius: var(--radius-2xl);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms;
	}
	.ap-btn--ghost {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.1);
	}
	.ap-btn--ghost:hover {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}
	.ap-rangeline {
		display: inline-flex;
		align-items: baseline;
		gap: 0.5rem;
		margin: 0 0 1.25rem;
		font-size: 0.75rem;
	}
	.ap-rangeline__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.ap-rangeline__value {
		color: var(--color-grey-300);
		font-variant-numeric: tabular-nums;
	}

	.ap-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 4rem 0;
		color: var(--color-grey-400);
		font-size: 0.875rem;
	}
	.ap-spinner {
		width: 1.25rem;
		height: 1.25rem;
		border: 2px solid rgba(255, 255, 255, 0.1);
		border-top-color: var(--color-teal);
		border-radius: 50%;
		animation: ap-spin 0.7s linear infinite;
	}
	@keyframes ap-spin {
		to {
			transform: rotate(360deg);
		}
	}
	.ap-error {
		padding: 0.85rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		line-height: 1.5;
		margin-bottom: 1rem;
	}

	.ap-kpis {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.85rem;
		margin-bottom: 1.5rem;
	}
	.ap-kpi {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 1.25rem;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.06);
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.ap-kpi__top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.ap-kpi__label {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1.2;
	}
	.ap-kpi__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.85rem;
		height: 1.85rem;
		border-radius: var(--radius-md);
		background: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
		flex-shrink: 0;
	}
	.ap-kpi__value {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		font-variant-numeric: tabular-nums lining-nums;
		letter-spacing: -0.02em;
		line-height: 1.1;
	}
	.ap-kpi__hint {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--color-grey-500);
		line-height: 1.4;
	}

	.ap-card {
		padding: 1.25rem;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.06);
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
		margin-bottom: 1.5rem;
	}
	.ap-card--full {
		width: 100%;
	}
	.ap-card__head {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 1rem;
	}
	.ap-card__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		line-height: 1;
	}
	.ap-card__title {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0;
		font-family: inherit;
		font-size: 1rem;
		font-weight: 600;
		letter-spacing: -0.005em;
		line-height: 1.3;
		color: var(--color-white);
	}

	.ap-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 2.5rem 1rem;
		text-align: center;
		color: var(--color-grey-500);
	}
	.ap-empty__title {
		margin: 0.5rem 0 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-white);
		line-height: 1.35;
	}
	.ap-empty__sub {
		margin: 0;
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--color-grey-500);
		line-height: 1.4;
	}
	.ap-empty code {
		font-size: 0.85em;
		padding: 0.1em 0.35em;
		border-radius: 0.25rem;
		background: rgba(255, 255, 255, 0.06);
	}

	.ap-bottom {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1.25rem;
		margin-bottom: 0;
	}
	.ap-bottom .ap-card {
		margin-bottom: 0;
	}

	.ap-table-wrap {
		overflow-x: auto;
		margin: -0.5rem;
		padding: 0.5rem;
	}
	.ap-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.875rem;
	}
	.ap-table th {
		text-align: left;
		padding: 0.65rem 1rem;
		color: var(--color-grey-500);
		font-weight: 700;
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background: rgba(255, 255, 255, 0.02);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.ap-table__num-th {
		text-align: right;
	}
	.ap-table td {
		padding: 0.875rem 1rem;
		color: var(--color-grey-300);
		font-size: 0.875rem;
		font-weight: 400;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		font-variant-numeric: tabular-nums;
	}
	.ap-table tr:hover td {
		background: rgba(255, 255, 255, 0.02);
	}
	.ap-table__path {
		color: var(--color-teal);
		font-weight: 500;
		word-break: break-all;
	}
	.ap-table__num {
		text-align: right;
		font-variant-numeric: tabular-nums;
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
	.ap-activity__item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
	}
	.ap-activity__dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--color-teal);
		margin-top: 0.45rem;
		flex-shrink: 0;
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}
	.ap-activity__body {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	.ap-activity__action {
		font-size: 0.875rem;
		color: var(--color-white);
		font-weight: 500;
		text-transform: capitalize;
		line-height: 1.5;
	}
	.ap-activity__meta {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--color-grey-500);
		line-height: 1.4;
	}

	@media (min-width: 480px) {
		.ap-kpis {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (min-width: 768px) {
		.ap-header {
			flex-direction: row;
			justify-content: space-between;
			align-items: flex-start;
			gap: 1.5rem;
		}
		.ap-toolbar {
			flex: 0 0 auto;
		}
		.ap-actions {
			margin-left: 0;
		}
		.ap-card {
			padding: 1.75rem;
			border-radius: var(--radius-2xl);
		}
		.ap-kpi {
			padding: 1.5rem;
			border-radius: var(--radius-2xl);
		}
		.ap-bottom {
			grid-template-columns: 1fr 1fr;
		}
	}
	@media (min-width: 1024px) {
		.ap-kpis {
			grid-template-columns: repeat(4, minmax(0, 1fr));
		}
	}
</style>
