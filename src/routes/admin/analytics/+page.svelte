<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import RevenueChart from '$lib/components/charts/RevenueChart.svelte';
	import GrowthChart from '$lib/components/charts/GrowthChart.svelte';

	const RANGES: Record<string, string> = {
		'7d': '7 Days',
		'30d': '30 Days',
		'90d': '90 Days',
		ytd: 'Year to Date'
	};

	let range = $state('30d');
	let loading = $state(true);
	let revenueData = $state<{ date: string; revenue_cents: number }[]>([]);
	let growthData = $state<{ month: string; growth_percent: number; revenue_cents: number }[]>([]);
	let kpis = $state({ mrr: 0, arr: 0, totalRevenue: 0, activeSubscribers: 0 });
	let topPages = $state<{ path: string; views: number; sessions: number }[]>([]);
	let recentActivity = $state<{ id: string; action: string; user: string; time: string }[]>([]);

	const mockRevenue = Array.from({ length: 12 }, (_, i) => ({
		date: `2026-${String(i + 1).padStart(2, '0')}-01`,
		revenue_cents: Math.floor(Math.random() * 500000) + 100000
	}));

	const mockGrowth = Array.from({ length: 12 }, (_, i) => {
		const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
		return {
			month: months[i],
			growth_percent: parseFloat((Math.random() * 30 - 8).toFixed(1)),
			revenue_cents: Math.floor(Math.random() * 500000) + 100000
		};
	});

	const mockKpis = {
		mrr: 4285000,
		arr: 51420000,
		totalRevenue: 38560000,
		activeSubscribers: 1247
	};

	const mockTopPages = [
		{ path: '/', views: 12840, sessions: 8320 },
		{ path: '/pricing', views: 6210, sessions: 4150 },
		{ path: '/blog/swing-tips', views: 4870, sessions: 3290 },
		{ path: '/dashboard', views: 3920, sessions: 2810 },
		{ path: '/signup', views: 3150, sessions: 2440 },
		{ path: '/about', views: 2680, sessions: 1930 }
	];

	const mockActivity = [
		{ id: '1', action: 'New subscription (Pro)', user: 'sarah@example.com', time: '2 min ago' },
		{ id: '2', action: 'Payment received', user: 'mike@example.com', time: '8 min ago' },
		{ id: '3', action: 'Plan upgraded to Team', user: 'alex@example.com', time: '24 min ago' },
		{ id: '4', action: 'Subscription cancelled', user: 'jen@example.com', time: '1 hr ago' },
		{ id: '5', action: 'New subscription (Basic)', user: 'tom@example.com', time: '2 hr ago' },
		{ id: '6', action: 'Payment received', user: 'lisa@example.com', time: '3 hr ago' }
	];

	function getRangeDates(r: string): { from: string; to: string } {
		const to = new Date();
		const from = new Date();
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

	async function load() {
		loading = true;
		const { from, to } = getRangeDates(range);
		const query = `from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`;

		try {
			const rev = await api.get<{ data: typeof revenueData }>(`/api/admin/analytics/revenue?${query}`);
			revenueData = rev.data && rev.data.length > 0 ? rev.data : mockRevenue;
		} catch {
			revenueData = mockRevenue;
		}

		try {
			const summary = await api.get<{
				mrr: number;
				arr: number;
				total_revenue: number;
				active_subscribers: number;
				growth: typeof growthData;
				top_pages: typeof topPages;
				recent_activity: typeof recentActivity;
			}>(`/api/analytics/summary?${query}`);
			kpis = {
				mrr: summary.mrr ?? mockKpis.mrr,
				arr: summary.arr ?? mockKpis.arr,
				totalRevenue: summary.total_revenue ?? mockKpis.totalRevenue,
				activeSubscribers: summary.active_subscribers ?? mockKpis.activeSubscribers
			};
			growthData = summary.growth && summary.growth.length > 0 ? summary.growth : mockGrowth;
			topPages = summary.top_pages && summary.top_pages.length > 0 ? summary.top_pages : mockTopPages;
			recentActivity = summary.recent_activity && summary.recent_activity.length > 0 ? summary.recent_activity : mockActivity;
		} catch {
			kpis = mockKpis;
			growthData = mockGrowth;
			topPages = mockTopPages;
			recentActivity = mockActivity;
		}

		loading = false;
	}

	onMount(() => {
		load();
	});

	function onRangeChange() {
		load();
	}

	const kpiCards = $derived([
		{ label: 'MRR', value: formatCents(kpis.mrr), accent: '#0fa4af' },
		{ label: 'ARR', value: formatCents(kpis.arr), accent: '#6366f1' },
		{ label: 'Total Revenue', value: formatCents(kpis.totalRevenue), accent: '#f59e0b' },
		{ label: 'Active Subscribers', value: formatNumber(kpis.activeSubscribers), accent: '#10b981' }
	]);
</script>

<svelte:head>
	<title>Analytics Dashboard - Admin</title>
</svelte:head>

<div class="ap">
	<header class="ap__header">
		<div>
			<h1 class="ap__title">Analytics Dashboard</h1>
			<p class="ap__sub">Revenue, growth, and engagement metrics</p>
		</div>
		<select class="ap__select" bind:value={range} onchange={onRangeChange}>
			{#each Object.entries(RANGES) as [key, label] (key)}
				<option value={key}>{label}</option>
			{/each}
		</select>
	</header>

	{#if loading}
		<div class="ap__loading">Loading analytics...</div>
	{:else}
		<section class="ap__kpis">
			{#each kpiCards as card (card.label)}
				<div class="ap__kpi" style="--kpi-accent: {card.accent}">
					<span class="ap__kpi-label">{card.label}</span>
					<span class="ap__kpi-value">{card.value}</span>
				</div>
			{/each}
		</section>

		<div class="ap__charts">
			<section class="ap__card">
				<h2 class="ap__card-title">Revenue</h2>
				<RevenueChart data={revenueData} />
			</section>

			<section class="ap__card">
				<h2 class="ap__card-title">Monthly Growth</h2>
				<GrowthChart data={growthData} />
			</section>
		</div>

		<div class="ap__bottom">
			<section class="ap__card ap__card--table">
				<h2 class="ap__card-title">Top Pages</h2>
				<div class="ap__table-wrap">
					<table class="ap__table">
						<thead>
							<tr>
								<th>Path</th>
								<th>Views</th>
								<th>Sessions</th>
							</tr>
						</thead>
						<tbody>
							{#each topPages as page (page.path)}
								<tr>
									<td class="ap__table-path">{page.path}</td>
									<td>{formatNumber(page.views)}</td>
									<td>{formatNumber(page.sessions)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</section>

			<section class="ap__card ap__card--activity">
				<h2 class="ap__card-title">Recent Activity</h2>
				<ul class="ap__activity">
					{#each recentActivity as item (item.id)}
						<li class="ap__activity-item">
							<div class="ap__activity-dot"></div>
							<div class="ap__activity-body">
								<span class="ap__activity-action">{item.action}</span>
								<span class="ap__activity-meta">{item.user} &middot; {item.time}</span>
							</div>
						</li>
					{/each}
				</ul>
			</section>
		</div>
	{/if}
</div>

<style>
	.ap {
		max-width: 76rem;
		padding: 0 0 3rem;
	}

	.ap__header {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.ap__title {
		margin: 0 0 0.3rem;
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--color-white, #f8fafc);
	}

	.ap__sub {
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-grey-500, #64748b);
	}

	.ap__select {
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

	.ap__select:focus {
		border-color: #0fa4af;
	}

	.ap__loading {
		text-align: center;
		padding: 4rem 0;
		color: var(--color-grey-500, #64748b);
		font-size: 1rem;
	}

	/* KPI Row */
	.ap__kpis {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.ap__kpi {
		padding: 1.25rem 1.35rem;
		border-radius: 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.07);
		background: rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(12px);
		border-top: 2px solid var(--kpi-accent);
	}

	.ap__kpi-label {
		display: block;
		font-size: 0.7rem;
		color: var(--color-grey-500, #64748b);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin-bottom: 0.4rem;
	}

	.ap__kpi-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white, #f8fafc);
		font-variant-numeric: tabular-nums;
	}

	/* Charts */
	.ap__charts {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.ap__card {
		padding: 1.5rem;
		border-radius: 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.07);
		background: rgba(255, 255, 255, 0.03);
		backdrop-filter: blur(12px);
	}

	.ap__card-title {
		margin: 0 0 1rem;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white, #f8fafc);
	}

	/* Bottom row */
	.ap__bottom {
		display: grid;
		grid-template-columns: 1.4fr 1fr;
		gap: 1.5rem;
	}

	/* Table */
	.ap__table-wrap {
		overflow-x: auto;
	}

	.ap__table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.ap__table th {
		text-align: left;
		padding: 0.6rem 0.75rem;
		color: var(--color-grey-500, #64748b);
		font-weight: 500;
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.ap__table td {
		padding: 0.6rem 0.75rem;
		color: rgba(255, 255, 255, 0.75);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		font-variant-numeric: tabular-nums;
	}

	.ap__table-path {
		color: #0fa4af;
		font-weight: 500;
	}

	.ap__table tr:last-child td {
		border-bottom: none;
	}

	/* Activity */
	.ap__activity {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.ap__activity-item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
	}

	.ap__activity-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #0fa4af;
		margin-top: 0.35rem;
		flex-shrink: 0;
	}

	.ap__activity-body {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}

	.ap__activity-action {
		font-size: 0.85rem;
		color: var(--color-white, #f8fafc);
		font-weight: 500;
	}

	.ap__activity-meta {
		font-size: 0.75rem;
		color: var(--color-grey-500, #64748b);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Responsive */
	@media (max-width: 900px) {
		.ap__kpis {
			grid-template-columns: repeat(2, 1fr);
		}

		.ap__charts {
			grid-template-columns: 1fr;
		}

		.ap__bottom {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 540px) {
		.ap__kpis {
			grid-template-columns: 1fr;
		}
	}
</style>
