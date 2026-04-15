<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { Watchlist, PaginatedResponse } from '$lib/api/types';
	import Plus from 'phosphor-svelte/lib/Plus';
	import Trash from 'phosphor-svelte/lib/Trash';
	import PencilSimple from 'phosphor-svelte/lib/PencilSimple';
	import Eye from 'phosphor-svelte/lib/Eye';
	import CaretLeft from 'phosphor-svelte/lib/CaretLeft';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';

	let watchlists = $state<Watchlist[]>([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	async function load() {
		loading = true;
		try {
			const res = await api.get<PaginatedResponse<Watchlist>>(
				`/api/admin/watchlists?page=${page}&per_page=15`
			);
			watchlists = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch {
			// handle
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function deleteWatchlist(wl: Watchlist) {
		if (!confirm(`Delete "${wl.title}"? This will also delete all its alerts.`)) return;
		try {
			await api.del(`/api/admin/watchlists/${wl.id}`);
			await load();
		} catch {
			alert('Failed to delete watchlist');
		}
	}

	async function togglePublish(wl: Watchlist) {
		try {
			await api.put(`/api/admin/watchlists/${wl.id}`, { published: !wl.published });
			await load();
		} catch {
			alert('Failed to update watchlist');
		}
	}
</script>

<svelte:head>
	<title>Watchlists - Admin - Precision Options Signals</title>
</svelte:head>

<div class="wl-admin">
	<div class="wl-admin__header">
		<div>
			<h1 class="wl-admin__title">Watchlists</h1>
			<p class="wl-admin__count">{total} total watchlists</p>
		</div>
		<a href="/admin/watchlists/new" class="wl-admin__create">
			<Plus size={18} weight="bold" />
			New Watchlist
		</a>
	</div>

	{#if loading}
		<p class="wl-admin__loading">Loading...</p>
	{:else if watchlists.length === 0}
		<div class="wl-admin__empty">
			<p>No watchlists created yet.</p>
			<a href="/admin/watchlists/new" class="wl-admin__create-link">Create your first watchlist →</a
			>
		</div>
	{:else}
		<!-- Mobile: Card view -->
		<div class="wl-admin__cards">
			{#each watchlists as wl (wl.id)}
				<div class="wl-card">
					<div class="wl-card__header">
						<span class="wl-card__title">{wl.title}</span>
						<span
							class={[
								'wl-card__status',
								wl.published ? 'wl-card__status--live' : 'wl-card__status--draft'
							]}
						>
							{wl.published ? 'Live' : 'Draft'}
						</span>
					</div>
					<div class="wl-card__row">
						<span class="wl-card__label">Week Of</span>
						<span class="wl-card__value">{wl.week_of}</span>
					</div>
					<div class="wl-card__row">
						<span class="wl-card__label">Published</span>
						<span class="wl-card__value"
							>{wl.published_at ? new Date(wl.published_at).toLocaleDateString() : '-'}</span
						>
					</div>
					<div class="wl-card__actions">
						<a href="/admin/watchlists/{wl.id}" class="wl-card__btn wl-card__btn--edit">
							<PencilSimple size={16} weight="bold" />
							<span>Edit</span>
						</a>
						<button onclick={() => togglePublish(wl)} class="wl-card__btn wl-card__btn--publish">
							<Eye size={16} weight="bold" />
							<span>{wl.published ? 'Unpublish' : 'Publish'}</span>
						</button>
						<button onclick={() => deleteWatchlist(wl)} class="wl-card__btn wl-card__btn--delete">
							<Trash size={16} weight="bold" />
							<span>Delete</span>
						</button>
					</div>
				</div>
			{/each}
		</div>
		<!-- Tablet+: Table view -->
		<div class="wl-admin__table-wrap">
			<table class="wl-table">
				<thead>
					<tr>
						<th>Title</th>
						<th>Week Of</th>
						<th>Status</th>
						<th>Published</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each watchlists as wl (wl.id)}
						<tr>
							<td class="wl-table__title">{wl.title}</td>
							<td>{wl.week_of}</td>
							<td>
								<span
									class={[
										'wl-table__status',
										wl.published ? 'wl-table__status--live' : 'wl-table__status--draft'
									]}
								>
									{wl.published ? 'Live' : 'Draft'}
								</span>
							</td>
							<td class="wl-table__date">
								{wl.published_at ? new Date(wl.published_at).toLocaleDateString() : '-'}
							</td>
							<td>
								<div class="wl-table__actions">
									<a
										href="/admin/watchlists/{wl.id}"
										class="wl-table__btn wl-table__btn--edit"
										title="Edit"
									>
										<PencilSimple size={16} weight="bold" />
									</a>
									<button
										onclick={() => togglePublish(wl)}
										class="wl-table__btn wl-table__btn--publish"
										title={wl.published ? 'Unpublish' : 'Publish'}
									>
										<Eye size={16} weight="bold" />
									</button>
									<button
										onclick={() => deleteWatchlist(wl)}
										class="wl-table__btn wl-table__btn--delete"
										title="Delete"
									>
										<Trash size={16} weight="bold" />
									</button>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<div class="wl-admin__pagination">
				<button
					onclick={() => {
						page--;
						load();
					}}
					disabled={page <= 1}
					class="wl-admin__page-btn"
				>
					<CaretLeft size={16} weight="bold" /> Prev
				</button>
				<span class="wl-admin__page-info">Page {page} of {totalPages}</span>
				<button
					onclick={() => {
						page++;
						load();
					}}
					disabled={page >= totalPages}
					class="wl-admin__page-btn"
				>
					Next <CaretRight size={16} weight="bold" />
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	/* Mobile-first base styles */
	.wl-admin__header {
		margin-bottom: 1rem;
	}

	.wl-admin__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.wl-admin__count {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-top: 0.15rem;
	}

	.wl-admin__create {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-xs);
		border-radius: var(--radius-lg);
		text-decoration: none;
		margin-top: 0.75rem;
		transition: opacity 200ms var(--ease-out);
	}

	.wl-admin__create:hover {
		opacity: 0.9;
	}

	.wl-admin__loading {
		color: var(--color-grey-400);
		text-align: center;
		padding: 2rem;
	}

	.wl-admin__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 2rem 1rem;
		background-color: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		color: var(--color-grey-400);
		text-align: center;
	}

	.wl-admin__create-link {
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		text-decoration: none;
	}

	/* Mobile: Card view */
	.wl-admin__cards {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.wl-admin__table-wrap {
		display: none;
	}

	.wl-card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
	}

	.wl-card__header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.wl-card__title {
		font-weight: var(--w-semibold);
		color: var(--color-white);
		font-size: var(--fs-base);
	}

	.wl-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.wl-card__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.wl-card__value {
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}

	.wl-card__status {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
	}

	.wl-card__status--live {
		background-color: rgba(34, 197, 94, 0.12);
		color: #22c55e;
	}

	.wl-card__status--draft {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}

	.wl-card__actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
		padding-top: 0.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.wl-card__btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.3rem;
		padding: 0.5rem 0.5rem;
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition: background-color 200ms var(--ease-out);
	}

	.wl-card__btn--edit {
		background-color: rgba(59, 130, 246, 0.1);
		color: #3b82f6;
	}

	.wl-card__btn--edit:hover {
		background-color: rgba(59, 130, 246, 0.25);
	}

	.wl-card__btn--publish {
		background-color: rgba(34, 197, 94, 0.1);
		color: #22c55e;
	}

	.wl-card__btn--publish:hover {
		background-color: rgba(34, 197, 94, 0.25);
	}

	.wl-card__btn--delete {
		background-color: rgba(239, 68, 68, 0.08);
		color: #ef4444;
	}

	.wl-card__btn--delete:hover {
		background-color: rgba(239, 68, 68, 0.2);
	}

	.wl-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.wl-admin__page-btn {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.5rem 0.75rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: border-color 200ms var(--ease-out);
	}

	.wl-admin__page-btn:hover:not(:disabled) {
		border-color: var(--color-teal);
	}

	.wl-admin__page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.wl-admin__page-info {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	/* Tablet+: Show table, hide cards */
	@media (min-width: 768px) {
		.wl-admin__header {
			display: flex;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 1.5rem;
		}

		.wl-admin__title {
			font-size: var(--fs-2xl);
		}

		.wl-admin__count {
			font-size: var(--fs-sm);
			margin-top: 0.25rem;
		}

		.wl-admin__create {
			margin-top: 0;
			padding: 0.65rem 1.25rem;
			font-size: var(--fs-sm);
		}

		.wl-admin__cards {
			display: none;
		}

		.wl-admin__table-wrap {
			display: block;
			overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
		}

		.wl-table {
			width: 100%;
			border-collapse: collapse;
		}

		.wl-table th {
			text-align: left;
			font-size: var(--fs-xs);
			font-weight: var(--w-semibold);
			color: var(--color-grey-400);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			padding: 0.85rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.wl-table td {
			padding: 0.85rem 1rem;
			font-size: var(--fs-sm);
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		}

		.wl-table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.wl-table__title {
			font-weight: var(--w-semibold);
			color: var(--color-white);
		}

		.wl-table__date {
			font-size: var(--fs-xs);
		}

		.wl-table__status {
			font-size: var(--fs-xs);
			font-weight: var(--w-semibold);
			padding: 0.15rem 0.55rem;
			border-radius: var(--radius-full);
		}

		.wl-table__status--live {
			background-color: rgba(34, 197, 94, 0.12);
			color: #22c55e;
		}

		.wl-table__status--draft {
			background-color: rgba(255, 255, 255, 0.06);
			color: var(--color-grey-400);
		}

		.wl-table__actions {
			display: flex;
			gap: 0.5rem;
		}

		.wl-table__btn {
			width: 2rem;
			height: 2rem;
			display: flex;
			align-items: center;
			justify-content: center;
			border-radius: var(--radius-lg);
			border: none;
			cursor: pointer;
			text-decoration: none;
			transition: background-color 200ms var(--ease-out);
		}

		.wl-table__btn--edit {
			background-color: rgba(59, 130, 246, 0.1);
			color: #3b82f6;
		}

		.wl-table__btn--edit:hover {
			background-color: rgba(59, 130, 246, 0.25);
		}

		.wl-table__btn--publish {
			background-color: rgba(34, 197, 94, 0.1);
			color: #22c55e;
		}

		.wl-table__btn--publish:hover {
			background-color: rgba(34, 197, 94, 0.25);
		}

		.wl-table__btn--delete {
			background-color: rgba(239, 68, 68, 0.08);
			color: #ef4444;
		}

		.wl-table__btn--delete:hover {
			background-color: rgba(239, 68, 68, 0.2);
		}

		.wl-admin__pagination {
			gap: 1rem;
			margin-top: 1.5rem;
		}

		.wl-admin__page-btn {
			gap: 0.35rem;
			padding: 0.5rem 1rem;
			font-size: var(--fs-sm);
		}

		.wl-admin__page-info {
			font-size: var(--fs-sm);
		}
	}
</style>
