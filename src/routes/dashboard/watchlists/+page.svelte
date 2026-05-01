<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import type { Watchlist, PaginatedResponse } from '$lib/api/types';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';

	const PER_PAGE = 6;

	let watchlists = $state<Watchlist[]>([]);
	let page = $state(1);
	let totalPages = $state(1);
	let totalCount = $state(0);
	let loading = $state(true);

	let rangeStart = $derived(totalCount === 0 ? 0 : (page - 1) * PER_PAGE + 1);
	let rangeEnd = $derived(Math.min(page * PER_PAGE, totalCount));

	function getEmbedUrl(url: string): string | null {
		if (!url) return null;
		const trimmed = url.trim();

		// YouTube — youtube.com/watch?v=ID
		const ytWatch = trimmed.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/watch\?(?:.*&)?v=([\w-]{6,})/);
		if (ytWatch) {
			return `https://www.youtube.com/embed/${ytWatch[1]}?rel=0&modestbranding=1`;
		}

		// YouTube — youtu.be/ID
		const ytShort = trimmed.match(/(?:https?:\/\/)?youtu\.be\/([\w-]{6,})/);
		if (ytShort) {
			return `https://www.youtube.com/embed/${ytShort[1]}?rel=0&modestbranding=1`;
		}

		// YouTube — youtube.com/embed/ID
		const ytEmbed = trimmed.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/embed\/([\w-]{6,})/);
		if (ytEmbed) {
			return `https://www.youtube.com/embed/${ytEmbed[1]}?rel=0&modestbranding=1`;
		}

		// Bunny.net
		if (/iframe\.mediadelivery\.net/.test(trimmed) || /video\.bunnycdn\.com/.test(trimmed)) {
			return trimmed;
		}

		return null;
	}

	async function loadWatchlists() {
		loading = true;
		try {
			const res = await api.get<PaginatedResponse<Watchlist>>(
				`/api/member/watchlists?page=${page}&per_page=${PER_PAGE}`
			);
			watchlists = res.data;
			totalPages = res.total_pages;
			totalCount = res.total;
		} catch {
			// handle error
		} finally {
			loading = false;
		}
	}

	onMount(loadWatchlists);

	function prevPage() {
		if (page > 1) {
			page--;
			loadWatchlists();
		}
	}

	function nextPage() {
		if (page < totalPages) {
			page++;
			loadWatchlists();
		}
	}
</script>

<svelte:head>
	<title>Watchlists - Precision Options Signals</title>
</svelte:head>

<div class="wl-page">
	<h2 class="wl-page__title">Weekly Watchlists</h2>
	<p class="wl-page__subtitle">Your Sunday night watchlists with entries, targets, and stops.</p>

	{#if loading}
		<p class="wl-page__loading">Loading watchlists...</p>
	{:else if watchlists.length === 0}
		<div class="wl-page__empty">
			<p>No watchlists published yet. Check back Sunday night!</p>
		</div>
	{:else}
		<div class="wl-page__grid">
			{#each watchlists as wl (wl.id)}
				{@const embedUrl = wl.video_url ? getEmbedUrl(wl.video_url) : null}
				<a href={resolve('/dashboard/watchlists/[id]', { id: wl.id })} class="wl-item">
					{#if embedUrl}
						<div class="wl-item__video">
							<iframe
								src={embedUrl}
								title={`${wl.title} video`}
								loading="lazy"
								allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
								allowfullscreen
							></iframe>
						</div>
					{/if}
					<div class="wl-item__body">
						<div class="wl-item__header">
							<h3 class="wl-item__title">{wl.title}</h3>
							<span class="wl-item__date">Week of {wl.week_of}</span>
						</div>
						{#if wl.notes}
							<p class="wl-item__notes">{wl.notes}</p>
						{/if}
						<div class="wl-item__footer">
							{#if wl.video_url && !embedUrl}
								<span class="wl-item__tag">Video included</span>
							{:else}
								<span></span>
							{/if}
							<span class="wl-item__arrow">View details →</span>
						</div>
					</div>
				</a>
			{/each}
		</div>

		{#if totalCount > 0}
			<p class="wl-page__range">
				Showing {rangeStart}–{rangeEnd} of {totalCount} watchlists
			</p>
		{/if}

		{#if totalPages > 1}
			<div class="wl-page__pagination">
				<button onclick={prevPage} disabled={page <= 1} class="wl-page__page-btn">
					<CaretLeftIcon size={16} weight="bold" /> Prev
				</button>
				<span class="wl-page__page-info">Page {page} of {totalPages}</span>
				<button onclick={nextPage} disabled={page >= totalPages} class="wl-page__page-btn">
					Next <CaretRightIcon size={16} weight="bold" />
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	.wl-page__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.wl-page__subtitle {
		color: var(--color-grey-400);
		margin-bottom: 2rem;
	}

	.wl-page__loading,
	.wl-page__empty {
		color: var(--color-grey-400);
		text-align: center;
		padding: 3rem;
		background-color: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		border: 1px dashed rgba(255, 255, 255, 0.1);
	}

	.wl-page__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1.25rem;
	}

	@media (min-width: 768px) {
		.wl-page__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.wl-item {
		display: flex;
		flex-direction: column;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-decoration: none;
		overflow: hidden;
		transition:
			border-color 200ms var(--ease-out),
			transform 200ms var(--ease-out);
	}

	.wl-item:hover {
		border-color: rgba(15, 164, 175, 0.3);
		transform: translateY(-2px);
	}

	.wl-item__video {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 9;
		background-color: var(--color-navy-deep);
	}

	.wl-item__video iframe {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: 0;
	}

	.wl-item__body {
		padding: 1.25rem 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		flex: 1;
	}

	.wl-item__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}

	.wl-item__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.wl-item__date {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		white-space: nowrap;
	}

	.wl-item__notes {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		line-height: 1.5;
	}

	.wl-item__footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: auto;
		padding-top: 0.5rem;
	}

	.wl-item__tag {
		font-size: var(--fs-xs);
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.wl-item__arrow {
		font-size: var(--fs-sm);
		color: var(--color-teal);
		font-weight: var(--w-semibold);
	}

	.wl-page__range {
		text-align: center;
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin-top: 1.5rem;
	}

	.wl-page__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 1rem;
	}

	.wl-page__page-btn {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.5rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		cursor: pointer;
		transition: border-color 200ms var(--ease-out);
	}

	.wl-page__page-btn:hover:not(:disabled) {
		border-color: var(--color-teal);
	}

	.wl-page__page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.wl-page__page-info {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
	}
</style>
