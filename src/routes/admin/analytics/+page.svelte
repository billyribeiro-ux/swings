<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AnalyticsSummary } from '$lib/api/types';

	function defaultRange(): { from: string; to: string } {
		const to = new Date();
		const from = new Date();
		from.setDate(from.getDate() - 30);
		const fmt = (d: Date) => d.toISOString().slice(0, 10);
		return { from: fmt(from), to: fmt(to) };
	}

	let from = $state(defaultRange().from);
	let to = $state(defaultRange().to);
	let summary = $state<AnalyticsSummary | null>(null);
	let loadError = $state('');
	let loading = $state(true);
	let Dashboard = $state<
		typeof import('$lib/components/admin/analytics/AnalyticsDashboard3d.svelte').default | null
	>(null);

	async function load() {
		loading = true;
		loadError = '';
		try {
			summary = await api.get<AnalyticsSummary>(
				`/api/admin/analytics/summary?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`
			);
		} catch (e) {
			summary = null;
			loadError = e instanceof ApiError ? e.message : 'Failed to load analytics';
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		const m = await import('$lib/components/admin/analytics/AnalyticsDashboard3d.svelte');
		Dashboard = m.default;
		await load();
	});
</script>

<svelte:head>
	<title>Analytics - Admin - Explosive Swings</title>
</svelte:head>

<div class="analytics-page">
	<div class="analytics-page__header">
		<div>
			<h1 class="analytics-page__title">Analytics</h1>
			<p class="analytics-page__sub">Site traffic, top pages, and CTA performance (Three.js)</p>
		</div>
		<form
			class="analytics-page__filters"
			onsubmit={(e) => {
				e.preventDefault();
				load();
			}}
		>
			<label class="analytics-page__label" for="analytics-range-from">
				<span>From</span>
				<input
					id="analytics-range-from"
					name="from"
					type="date"
					bind:value={from}
					class="analytics-page__input"
					autocomplete="off"
				/>
			</label>
			<label class="analytics-page__label" for="analytics-range-to">
				<span>To</span>
				<input
					id="analytics-range-to"
					name="to"
					type="date"
					bind:value={to}
					class="analytics-page__input"
					autocomplete="off"
				/>
			</label>
			<button type="submit" class="analytics-page__btn" disabled={loading}>Apply</button>
		</form>
	</div>

	{#if loading}
		<p class="analytics-page__loading">Loading analytics…</p>
	{:else if loadError}
		<p class="analytics-page__error">{loadError}</p>
	{:else if summary}
		<div class="analytics-page__kpis">
			<div class="analytics-page__kpi">
				<span class="analytics-page__kpi-label">Page views</span>
				<span class="analytics-page__kpi-value">{summary.total_page_views}</span>
			</div>
			<div class="analytics-page__kpi">
				<span class="analytics-page__kpi-label">Sessions</span>
				<span class="analytics-page__kpi-value">{summary.total_sessions}</span>
			</div>
			<div class="analytics-page__kpi analytics-page__kpi--muted">
				<span class="analytics-page__kpi-label">Range</span>
				<span class="analytics-page__kpi-value analytics-page__kpi-range"
					>{summary.from} → {summary.to}</span
				>
			</div>
		</div>

		{#if Dashboard && summary}
			{#key `${summary.from}-${summary.to}`}
				<Dashboard {summary} />
			{/key}
		{/if}
	{/if}
</div>

<style>
	.analytics-page {
		max-width: 72rem;
	}

	.analytics-page__header {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.analytics-page__title {
		margin: 0 0 0.35rem;
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--color-white);
	}

	.analytics-page__sub {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--color-grey-500);
	}

	.analytics-page__filters {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: 0.75rem;
	}

	.analytics-page__label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
	}

	.analytics-page__input {
		padding: 0.5rem 0.65rem;
		border-radius: var(--radius-md);
		border: 1px solid rgba(255, 255, 255, 0.12);
		background: rgba(0, 0, 0, 0.35);
		color: var(--color-white);
		font-family: inherit;
	}

	.analytics-page__btn {
		padding: 0.5rem 1rem;
		border-radius: var(--radius-md);
		border: none;
		background: var(--color-teal, #0fa4af);
		color: var(--color-navy, #0f172a);
		font-weight: 600;
		cursor: pointer;
	}

	.analytics-page__btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.analytics-page__loading,
	.analytics-page__error {
		color: var(--color-grey-400);
	}

	.analytics-page__error {
		color: #f87171;
	}

	.analytics-page__kpis {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.analytics-page__kpi {
		min-width: 10rem;
		padding: 1rem 1.25rem;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(255, 255, 255, 0.04);
	}

	.analytics-page__kpi--muted {
		flex: 1;
		min-width: 14rem;
	}

	.analytics-page__kpi-label {
		display: block;
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 0.35rem;
	}

	.analytics-page__kpi-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		font-variant-numeric: tabular-nums;
	}

	.analytics-page__kpi-range {
		font-size: var(--fs-sm);
		font-weight: 500;
	}
</style>
