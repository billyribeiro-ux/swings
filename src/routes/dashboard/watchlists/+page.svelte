<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { Watchlist, PaginatedResponse } from '$lib/api/types';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';

	let watchlists = $state<Watchlist[]>([]);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	async function loadWatchlists() {
		loading = true;
		try {
			const res = await api.get<PaginatedResponse<Watchlist>>(
				`/api/member/watchlists?page=${page}&per_page=10`
			);
			watchlists = res.data;
			totalPages = res.total_pages;
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
				<a href="/dashboard/watchlists/{wl.id}" class="wl-item">
					<div class="wl-item__header">
						<h3 class="wl-item__title">{wl.title}</h3>
						<span class="wl-item__date">Week of {wl.week_of}</span>
					</div>
					{#if wl.notes}
						<p class="wl-item__notes">{wl.notes}</p>
					{/if}
					<div class="wl-item__footer">
						{#if wl.video_url}
							<span class="wl-item__tag">Video included</span>
						{/if}
						<span class="wl-item__arrow">View details →</span>
					</div>
				</a>
			{/each}
		</div>

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
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.wl-item {
		display: block;
		padding: 1.25rem 1.5rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-decoration: none;
		transition:
			border-color 200ms var(--ease-out),
			transform 200ms var(--ease-out);
	}

	.wl-item:hover {
		border-color: rgba(15, 164, 175, 0.3);
		transform: translateY(-2px);
	}

	.wl-item__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.5rem;
	}

	.wl-item__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.wl-item__date {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.wl-item__notes {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin-bottom: 0.75rem;
		line-height: 1.5;
	}

	.wl-item__footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.wl-item__tag {
		font-size: var(--fs-xs);
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		background-color: rgba(168, 85, 247, 0.15);
		color: #a855f7;
	}

	.wl-item__arrow {
		font-size: var(--fs-sm);
		color: var(--color-teal);
		font-weight: var(--w-semibold);
	}

	.wl-page__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 2rem;
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
