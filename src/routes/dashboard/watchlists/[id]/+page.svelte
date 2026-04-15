<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { WatchlistWithAlerts } from '$lib/api/types';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';
	import TrendDown from 'phosphor-svelte/lib/TrendDown';

	let watchlist = $state<WatchlistWithAlerts | null>(null);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		try {
			const id = page.params.id;
			watchlist = await api.get<WatchlistWithAlerts>(`/api/member/watchlists/${id}`);
		} catch {
			error = 'Watchlist not found or unavailable.';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{watchlist?.title ?? 'Watchlist'} - Precision Options Signals</title>
</svelte:head>

<div class="wl-detail">
	<a href="/dashboard/watchlists" class="wl-detail__back">
		<ArrowLeft size={18} />
		Back to Watchlists
	</a>

	{#if loading}
		<p class="wl-detail__loading">Loading...</p>
	{:else if error}
		<p class="wl-detail__error">{error}</p>
	{:else if watchlist}
		<div class="wl-detail__header">
			<h1 class="wl-detail__title">{watchlist.title}</h1>
			<span class="wl-detail__date">Week of {watchlist.week_of}</span>
		</div>

		{#if watchlist.notes}
			<p class="wl-detail__notes">{watchlist.notes}</p>
		{/if}

		{#if watchlist.video_url}
			<div class="wl-detail__video">
				<a href={watchlist.video_url} target="_blank" rel="noopener" class="wl-detail__video-link">
					Watch Video Walkthrough →
				</a>
			</div>
		{/if}

		<!-- Alerts -->
		<div class="wl-detail__alerts">
			<h2 class="wl-detail__section-title">Alerts ({watchlist.alerts.length})</h2>

			{#if watchlist.alerts.length === 0}
				<p class="wl-detail__empty">No alerts in this watchlist yet.</p>
			{:else}
				<div class="alerts-grid">
					{#each watchlist.alerts as alert (alert.id)}
						<div class="alert-card">
							<div class="alert-card__header">
								<div class="alert-card__ticker-wrap">
									{#if alert.direction === 'bullish'}
										<TrendUp size={18} weight="bold" class="alert-card__icon--bull" />
									{:else}
										<TrendDown size={18} weight="bold" class="alert-card__icon--bear" />
									{/if}
									<h3 class="alert-card__ticker">{alert.ticker}</h3>
								</div>
								<span
									class={[
										'alert-card__direction',
										alert.direction === 'bullish'
											? 'alert-card__direction--bull'
											: 'alert-card__direction--bear'
									]}
								>
									{alert.direction}
								</span>
							</div>

							<div class="alert-card__levels">
								<div class="alert-card__level">
									<span class="alert-card__level-label">Entry Zone</span>
									<span class="alert-card__level-value">{alert.entry_zone}</span>
								</div>
								<div class="alert-card__level">
									<span class="alert-card__level-label">Invalidation</span>
									<span class="alert-card__level-value alert-card__level-value--red">
										{alert.invalidation}
									</span>
								</div>
								<div class="alert-card__level">
									<span class="alert-card__level-label">Profit Targets</span>
									<span class="alert-card__level-value alert-card__level-value--green">
										{alert.profit_zones.join(' / ')}
									</span>
								</div>
							</div>

							{#if alert.notes}
								<p class="alert-card__notes">{alert.notes}</p>
							{/if}

							{#if alert.chart_url}
								<a
									href={alert.chart_url}
									target="_blank"
									rel="noopener"
									class="alert-card__chart-link"
								>
									View Chart →
								</a>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.wl-detail__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}

	.wl-detail__back:hover {
		color: var(--color-white);
	}

	.wl-detail__loading,
	.wl-detail__error {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400);
	}

	.wl-detail__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.wl-detail__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.wl-detail__date {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		background-color: var(--color-navy-mid);
		padding: 0.35rem 0.85rem;
		border-radius: var(--radius-full);
	}

	.wl-detail__notes {
		color: var(--color-grey-300);
		line-height: 1.65;
		margin-bottom: 1.5rem;
	}

	.wl-detail__video {
		margin-bottom: 2rem;
	}

	.wl-detail__video-link {
		display: inline-flex;
		padding: 0.65rem 1.25rem;
		background-color: rgba(168, 85, 247, 0.12);
		border: 1px solid rgba(168, 85, 247, 0.3);
		border-radius: var(--radius-lg);
		color: #c084fc;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: background-color 200ms var(--ease-out);
	}

	.wl-detail__video-link:hover {
		background-color: rgba(168, 85, 247, 0.2);
	}

	.wl-detail__section-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 1rem;
	}

	.wl-detail__empty {
		color: var(--color-grey-400);
		text-align: center;
		padding: 2rem;
	}

	.alerts-grid {
		display: grid;
		gap: 1rem;
	}

	@media (min-width: 768px) {
		.alerts-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.alert-card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
	}

	.alert-card__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}

	.alert-card__ticker-wrap {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.alert-card__ticker {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	:global(.alert-card__icon--bull) {
		color: #22c55e;
	}

	:global(.alert-card__icon--bear) {
		color: #ef4444;
	}

	.alert-card__direction {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		padding: 0.2rem 0.65rem;
		border-radius: var(--radius-full);
		text-transform: capitalize;
	}

	.alert-card__direction--bull {
		background-color: rgba(34, 197, 94, 0.12);
		color: #22c55e;
	}

	.alert-card__direction--bear {
		background-color: rgba(239, 68, 68, 0.12);
		color: #ef4444;
	}

	.alert-card__levels {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
		margin-bottom: 1rem;
	}

	.alert-card__level {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.alert-card__level-label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.alert-card__level-value {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.alert-card__level-value--red {
		color: #fca5a5;
	}

	.alert-card__level-value--green {
		color: #86efac;
	}

	.alert-card__notes {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		line-height: 1.5;
		margin-bottom: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.alert-card__chart-link {
		font-size: var(--fs-sm);
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	.alert-card__chart-link:hover {
		text-decoration: underline;
	}
</style>
