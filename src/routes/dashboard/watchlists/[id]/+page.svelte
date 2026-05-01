<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { WatchlistWithAlerts } from '$lib/api/types';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import TrendUpIcon from 'phosphor-svelte/lib/TrendUpIcon';
	import TrendDownIcon from 'phosphor-svelte/lib/TrendDownIcon';
	import ChartLineIcon from 'phosphor-svelte/lib/ChartLineIcon';
	import VideoCameraIcon from 'phosphor-svelte/lib/VideoCameraIcon';

	type DirectionFilter = 'all' | 'bullish' | 'bearish';

	let watchlist = $state<WatchlistWithAlerts | null>(null);
	let loading = $state(true);
	let error = $state('');
	let directionFilter = $state<DirectionFilter>('all');

	function getEmbedUrl(url: string): string | null {
		if (!url) return null;
		const trimmed = url.trim();

		const ytWatch = trimmed.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/watch\?(?:.*&)?v=([\w-]{6,})/);
		if (ytWatch) {
			return `https://www.youtube.com/embed/${ytWatch[1]}?rel=0&modestbranding=1`;
		}

		const ytShort = trimmed.match(/(?:https?:\/\/)?youtu\.be\/([\w-]{6,})/);
		if (ytShort) {
			return `https://www.youtube.com/embed/${ytShort[1]}?rel=0&modestbranding=1`;
		}

		const ytEmbed = trimmed.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/embed\/([\w-]{6,})/);
		if (ytEmbed) {
			return `https://www.youtube.com/embed/${ytEmbed[1]}?rel=0&modestbranding=1`;
		}

		if (/iframe\.mediadelivery\.net/.test(trimmed) || /video\.bunnycdn\.com/.test(trimmed)) {
			return trimmed;
		}

		return null;
	}

	let alerts = $derived(watchlist?.alerts ?? []);
	let bullishCount = $derived(alerts.filter((a) => a.direction === 'bullish').length);
	let bearishCount = $derived(alerts.filter((a) => a.direction === 'bearish').length);
	let totalCount = $derived(alerts.length);
	let filteredAlerts = $derived(
		directionFilter === 'all' ? alerts : alerts.filter((a) => a.direction === directionFilter)
	);
	let videoEmbedUrl = $derived(watchlist?.video_url ? getEmbedUrl(watchlist.video_url) : null);

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
	<a href={resolve('/dashboard/watchlists')} class="wl-detail__back">
		<ArrowLeftIcon size={18} />
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
			<section class="wl-detail__video-section">
				<h2 class="wl-detail__section-title">
					<VideoCameraIcon size={20} weight="bold" />
					Video Walkthrough
				</h2>
				{#if videoEmbedUrl}
					<div class="wl-detail__video-embed">
						<iframe
							src={videoEmbedUrl}
							title={`${watchlist.title} walkthrough`}
							loading="lazy"
							allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
							allowfullscreen
						></iframe>
					</div>
				{:else}
					<!-- eslint-disable svelte/no-navigation-without-resolve -- video_url is admin-supplied (typically YouTube/Vimeo) and opens in a new tab; resolve() does not apply -->
					<a
						href={watchlist.video_url}
						target="_blank"
						rel="noopener"
						class="wl-detail__video-link"
					>
						Watch Video Walkthrough →
					</a>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				{/if}
			</section>
		{/if}

		<!-- Alerts -->
		<div class="wl-detail__alerts">
			<h2 class="wl-detail__section-title">Alerts ({totalCount})</h2>

			{#if alerts.length === 0}
				<p class="wl-detail__empty">No alerts in this watchlist yet.</p>
			{:else}
				<div class="wl-detail__filters" role="tablist" aria-label="Filter alerts by direction">
					<button
						type="button"
						role="tab"
						aria-selected={directionFilter === 'all'}
						class={[
							'wl-detail__filter',
							directionFilter === 'all' && 'wl-detail__filter--active'
						]}
						onclick={() => (directionFilter = 'all')}
					>
						All ({totalCount})
					</button>
					<button
						type="button"
						role="tab"
						aria-selected={directionFilter === 'bullish'}
						class={[
							'wl-detail__filter',
							'wl-detail__filter--bull',
							directionFilter === 'bullish' && 'wl-detail__filter--active'
						]}
						onclick={() => (directionFilter = 'bullish')}
					>
						<TrendUpIcon size={14} weight="bold" />
						Bullish ({bullishCount})
					</button>
					<button
						type="button"
						role="tab"
						aria-selected={directionFilter === 'bearish'}
						class={[
							'wl-detail__filter',
							'wl-detail__filter--bear',
							directionFilter === 'bearish' && 'wl-detail__filter--active'
						]}
						onclick={() => (directionFilter = 'bearish')}
					>
						<TrendDownIcon size={14} weight="bold" />
						Bearish ({bearishCount})
					</button>
				</div>

				{#if filteredAlerts.length === 0}
					<p class="wl-detail__empty">No {directionFilter} alerts in this watchlist.</p>
				{:else}
					<div class="alerts-grid">
						{#each filteredAlerts as alert (alert.id)}
							<article
								class={[
									'alert-card',
									alert.direction === 'bullish'
										? 'alert-card--bull'
										: 'alert-card--bear'
								]}
							>
								<div class="alert-card__header">
									<h3 class="alert-card__ticker">{alert.ticker}</h3>
									<span
										class={[
											'alert-card__direction',
											alert.direction === 'bullish'
												? 'alert-card__direction--bull'
												: 'alert-card__direction--bear'
										]}
									>
										{#if alert.direction === 'bullish'}
											<TrendUpIcon size={14} weight="bold" />
											Bullish
										{:else}
											<TrendDownIcon size={14} weight="bold" />
											Bearish
										{/if}
									</span>
								</div>

								<dl class="alert-card__levels">
									<div class="alert-card__level">
										<dt class="alert-card__level-label">Entry Zone</dt>
										<dd class="alert-card__level-value">{alert.entry_zone}</dd>
									</div>
									<div class="alert-card__level">
										<dt class="alert-card__level-label">Invalidation</dt>
										<dd class="alert-card__level-value alert-card__level-value--red">
											{alert.invalidation}
										</dd>
									</div>
									<div class="alert-card__level alert-card__level--block">
										<dt class="alert-card__level-label">Profit Targets</dt>
										<dd class="alert-card__chips">
											{#each alert.profit_zones as target, i (i)}
												<span class="alert-card__chip">{target}</span>
											{/each}
										</dd>
									</div>
								</dl>

								{#if alert.notes}
									<p class="alert-card__notes">{alert.notes}</p>
								{/if}

								{#if alert.chart_url}
									<!-- eslint-disable svelte/no-navigation-without-resolve -- chart_url is admin-supplied (typically TradingView) and opens in a new tab; resolve() does not apply -->
									<a
										href={alert.chart_url}
										target="_blank"
										rel="noopener"
										class="alert-card__chart-btn"
									>
										<ChartLineIcon size={16} weight="bold" />
										View Chart
									</a>
									<!-- eslint-enable svelte/no-navigation-without-resolve -->
								{/if}
							</article>
						{/each}
					</div>
				{/if}
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

	.wl-detail__video-section {
		margin-bottom: 2rem;
	}

	.wl-detail__video-embed {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 9;
		background-color: var(--color-navy-deep);
		border-radius: var(--radius-xl);
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.06);
	}

	.wl-detail__video-embed iframe {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: 0;
	}

	.wl-detail__video-link {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.65rem 1.25rem;
		background-color: rgba(15, 164, 175, 0.12);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-lg);
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: background-color 200ms var(--ease-out);
	}

	.wl-detail__video-link:hover {
		background-color: rgba(15, 164, 175, 0.2);
	}

	.wl-detail__section-title {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
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

	.wl-detail__filters {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.wl-detail__filter {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.45rem 0.9rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-full);
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			color 150ms var(--ease-out);
	}

	.wl-detail__filter:hover {
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}

	.wl-detail__filter--active {
		background-color: var(--color-teal);
		border-color: var(--color-teal);
		color: var(--color-navy-deep);
	}

	.wl-detail__filter--bull.wl-detail__filter--active {
		background-color: var(--color-teal);
		border-color: var(--color-teal);
		color: var(--color-navy-deep);
	}

	.wl-detail__filter--bear.wl-detail__filter--active {
		background-color: oklch(0.62 0.2 25);
		border-color: oklch(0.62 0.2 25);
		color: var(--color-white);
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
		border-left-width: 3px;
		border-radius: var(--radius-xl);
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.alert-card--bull {
		border-left-color: var(--color-teal);
		background-image: linear-gradient(
			to right,
			rgba(15, 164, 175, 0.08),
			transparent 35%
		);
	}

	.alert-card--bear {
		border-left-color: oklch(0.62 0.2 25);
		background-image: linear-gradient(
			to right,
			oklch(0.62 0.2 25 / 0.1),
			transparent 35%
		);
	}

	.alert-card__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.alert-card__ticker {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		letter-spacing: 0.02em;
	}

	.alert-card__direction {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		padding: 0.25rem 0.65rem;
		border-radius: var(--radius-full);
	}

	.alert-card__direction--bull {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.alert-card__direction--bear {
		background-color: oklch(0.62 0.2 25 / 0.15);
		color: oklch(0.78 0.16 25);
	}

	.alert-card__levels {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
		margin: 0;
	}

	.alert-card__level {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.75rem;
	}

	.alert-card__level--block {
		flex-direction: column;
		align-items: flex-start;
		gap: 0.4rem;
	}

	.alert-card__level-label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.alert-card__level-value {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0;
		text-align: right;
	}

	.alert-card__level-value--red {
		color: oklch(0.78 0.16 25);
	}

	.alert-card__chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin: 0;
	}

	.alert-card__chip {
		display: inline-flex;
		align-items: center;
		padding: 0.2rem 0.65rem;
		border-radius: var(--radius-full);
		background-color: rgba(34, 197, 94, 0.12);
		color: var(--color-green, #86efac);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
	}

	.alert-card__notes {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		line-height: 1.5;
		padding-top: 0.75rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.alert-card__chart-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		align-self: flex-start;
		margin-top: auto;
		padding: 0.5rem 0.9rem;
		background-color: rgba(15, 164, 175, 0.12);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-lg);
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: background-color 200ms var(--ease-out);
	}

	.alert-card__chart-btn:hover {
		background-color: rgba(15, 164, 175, 0.22);
	}
</style>
