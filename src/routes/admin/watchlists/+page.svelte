<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { Watchlist, PaginatedResponse } from '$lib/api/types';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import EyeSlashIcon from 'phosphor-svelte/lib/EyeSlashIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';

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

	function formatDate(dateStr: string | null | undefined): string {
		if (!dateStr) return '—';
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Watchlists - Admin - Precision Options Signals</title>
</svelte:head>

<div class="wl-admin">
	<header class="wl-admin__page-header">
		<div class="wl-admin__heading">
			<span class="wl-admin__eyebrow">Publishing</span>
			<h1 class="wl-admin__title">Watchlists</h1>
			<p class="wl-admin__subtitle">
				{total.toLocaleString()} total {total === 1 ? 'watchlist' : 'watchlists'} — schedule, publish, and manage weekly issues.
			</p>
		</div>
		<a href="/admin/watchlists/new" class="wl-admin__create">
			<PlusIcon size={16} weight="bold" />
			<span>New watchlist</span>
		</a>
	</header>

	{#if loading}
		<div class="wl-admin__skeleton" aria-hidden="true">
			{#each Array(4) as _, i (i)}
				<div class="wl-admin__skeleton-card"></div>
			{/each}
		</div>
	{:else if watchlists.length === 0}
		<div class="wl-admin__empty">
			<ListChecksIcon size={48} weight="duotone" color="var(--color-grey-500)" />
			<p class="wl-admin__empty-title">No watchlists yet</p>
			<p class="wl-admin__empty-body">
				Watchlists drive your weekly subscriber alerts. Publish your first one to get started.
			</p>
			<a href="/admin/watchlists/new" class="wl-admin__create wl-admin__create--empty">
				<PlusIcon size={16} weight="bold" />
				<span>Create your first watchlist</span>
			</a>
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
							<span class="wl-card__status-dot"></span>
							{wl.published ? 'Live' : 'Draft'}
						</span>
					</div>
					<div class="wl-card__row">
						<span class="wl-card__label">Week of</span>
						<span class="wl-card__value">{wl.week_of}</span>
					</div>
					<div class="wl-card__row">
						<span class="wl-card__label">Published</span>
						<span class="wl-card__value">{formatDate(wl.published_at)}</span>
					</div>
					<div class="wl-card__actions">
						<a href="/admin/watchlists/{wl.id}" class="wl-card__btn wl-card__btn--edit" title="Edit">
							<PencilSimpleIcon size={16} weight="bold" />
							<span>Edit</span>
						</a>
						<button
							onclick={() => togglePublish(wl)}
							class="wl-card__btn wl-card__btn--publish"
							title={wl.published ? 'Unpublish' : 'Publish'}
						>
							{#if wl.published}
								<EyeSlashIcon size={16} weight="bold" />
								<span>Unpublish</span>
							{:else}
								<EyeIcon size={16} weight="bold" />
								<span>Publish</span>
							{/if}
						</button>
						<button
							onclick={() => deleteWatchlist(wl)}
							class="wl-card__btn wl-card__btn--delete"
							title="Delete"
						>
							<TrashIcon size={16} weight="bold" />
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
						<th>Week of</th>
						<th>Status</th>
						<th>Published</th>
						<th class="wl-table__th-actions">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each watchlists as wl (wl.id)}
						<tr>
							<td class="wl-table__title">{wl.title}</td>
							<td class="wl-table__muted">{wl.week_of}</td>
							<td>
								<span
									class={[
										'wl-table__status',
										wl.published ? 'wl-table__status--live' : 'wl-table__status--draft'
									]}
								>
									<span class="wl-table__status-dot"></span>
									{wl.published ? 'Live' : 'Draft'}
								</span>
							</td>
							<td class="wl-table__muted">{formatDate(wl.published_at)}</td>
							<td>
								<div class="wl-table__actions">
									<a
										href="/admin/watchlists/{wl.id}"
										class="wl-table__btn wl-table__btn--edit"
										title="Edit"
										aria-label="Edit"
									>
										<PencilSimpleIcon size={16} weight="bold" />
									</a>
									<button
										onclick={() => togglePublish(wl)}
										class="wl-table__btn wl-table__btn--publish"
										title={wl.published ? 'Unpublish' : 'Publish'}
										aria-label={wl.published ? 'Unpublish' : 'Publish'}
									>
										{#if wl.published}
											<EyeSlashIcon size={16} weight="bold" />
										{:else}
											<EyeIcon size={16} weight="bold" />
										{/if}
									</button>
									<button
										onclick={() => deleteWatchlist(wl)}
										class="wl-table__btn wl-table__btn--delete"
										title="Delete"
										aria-label="Delete"
									>
										<TrashIcon size={16} weight="bold" />
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
					aria-label="Previous page"
				>
					<CaretLeftIcon size={16} weight="bold" />
					<span>Prev</span>
				</button>
				<span class="wl-admin__page-info">Page {page} of {totalPages}</span>
				<button
					onclick={() => {
						page++;
						load();
					}}
					disabled={page >= totalPages}
					class="wl-admin__page-btn"
					aria-label="Next page"
				>
					<span>Next</span>
					<CaretRightIcon size={16} weight="bold" />
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	@keyframes shimmer {
		0% {
			background-position: -200% 0;
		}
		100% {
			background-position: 200% 0;
		}
	}

	/* ====================================================================
	   PAGE HEADER
	   ==================================================================== */
	.wl-admin__page-header {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		margin-bottom: 1.25rem;
	}

	.wl-admin__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.wl-admin__title {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.15;
		letter-spacing: -0.01em;
		margin: 0.25rem 0 0.4rem;
	}

	.wl-admin__subtitle {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 60ch;
		line-height: 1.55;
		margin: 0;
	}

	.wl-admin__create {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0.65rem 1rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		color: var(--color-white);
		font-weight: 600;
		font-size: 0.875rem;
		border-radius: var(--radius-lg);
		text-decoration: none;
		border: none;
		cursor: pointer;
		align-self: flex-start;
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
		transition: all 200ms var(--ease-out);
	}

	.wl-admin__create:hover {
		background: linear-gradient(135deg, var(--color-teal-light), var(--color-teal));
		transform: translateY(-1px);
		box-shadow: 0 8px 20px -4px rgba(15, 164, 175, 0.55);
	}

	.wl-admin__create--empty {
		margin-top: 0.5rem;
	}

	/* ====================================================================
	   SKELETON / EMPTY
	   ==================================================================== */
	.wl-admin__skeleton {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.wl-admin__skeleton-card {
		height: 110px;
		border-radius: var(--radius-xl);
		background: linear-gradient(
			90deg,
			rgba(255, 255, 255, 0.03) 0%,
			rgba(255, 255, 255, 0.06) 50%,
			rgba(255, 255, 255, 0.03) 100%
		);
		background-size: 200% 100%;
		animation: shimmer 1.6s ease-in-out infinite;
	}

	.wl-admin__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 3rem 1rem;
		background-color: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		text-align: center;
	}

	.wl-admin__empty-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0.5rem 0 0;
	}

	.wl-admin__empty-body {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		margin: 0;
		max-width: 38ch;
		line-height: 1.55;
	}

	/* ====================================================================
	   MOBILE — card view
	   ==================================================================== */
	.wl-admin__cards {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.wl-admin__table-wrap {
		display: none;
	}

	.wl-card {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		padding: 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		transition: all 200ms var(--ease-out);
	}

	.wl-card:hover {
		border-color: rgba(255, 255, 255, 0.1);
		transform: translateY(-1px);
	}

	.wl-card__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.wl-card__title {
		font-weight: 600;
		color: var(--color-white);
		font-size: 1rem;
		line-height: 1.3;
	}

	.wl-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.wl-card__label {
		font-size: 0.75rem;
		color: var(--color-grey-500);
		font-weight: 500;
	}

	.wl-card__value {
		font-size: 0.875rem;
		color: var(--color-grey-300);
	}

	.wl-card__status {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.75rem;
		font-weight: 600;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		flex-shrink: 0;
	}

	.wl-card__status-dot {
		width: 0.4rem;
		height: 0.4rem;
		border-radius: var(--radius-full);
		background: currentColor;
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
		margin-top: 0.25rem;
		padding-top: 0.625rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.wl-card__btn {
		flex: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		min-height: 2.25rem;
		padding: 0.5rem 0.5rem;
		border-radius: var(--radius-lg);
		border: 1px solid transparent;
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 600;
		text-decoration: none;
		transition: all 150ms var(--ease-out);
	}

	.wl-card__btn--edit {
		background-color: rgba(59, 130, 246, 0.1);
		color: #60a5fa;
	}

	.wl-card__btn--edit:hover {
		background-color: rgba(59, 130, 246, 0.22);
	}

	.wl-card__btn--publish {
		background-color: rgba(34, 197, 94, 0.1);
		color: #22c55e;
	}

	.wl-card__btn--publish:hover {
		background-color: rgba(34, 197, 94, 0.22);
	}

	.wl-card__btn--delete {
		background-color: rgba(239, 68, 68, 0.08);
		color: #ef4444;
	}

	.wl-card__btn--delete:hover {
		background-color: rgba(239, 68, 68, 0.2);
	}

	/* ====================================================================
	   PAGINATION
	   ==================================================================== */
	.wl-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1.25rem;
	}

	.wl-admin__page-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		min-height: 2.25rem;
		padding: 0.5rem 0.875rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}

	.wl-admin__page-btn:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.08);
		border-color: rgba(15, 164, 175, 0.4);
	}

	.wl-admin__page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.wl-admin__page-info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
	}

	/* ====================================================================
	   TABLET+ (>=768px) — table view
	   ==================================================================== */
	@media (min-width: 768px) {
		.wl-admin__page-header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
			margin-bottom: 1.5rem;
		}


		.wl-admin__create {
			align-self: flex-end;
		}

		.wl-admin__cards {
			display: none;
		}

		.wl-admin__table-wrap {
			display: block;
			overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-2xl);
			box-shadow:
				0 1px 0 rgba(255, 255, 255, 0.03) inset,
				0 12px 32px rgba(0, 0, 0, 0.18);
		}

		.wl-table {
			width: 100%;
			border-collapse: collapse;
		}

		.wl-table thead {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.wl-table th {
			text-align: left;
			font-size: 0.6875rem;
			font-weight: 600;
			color: var(--color-grey-500);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			padding: 0.875rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.wl-table__th-actions {
			text-align: right;
		}

		.wl-table td {
			padding: 0.875rem 1rem;
			font-size: 0.875rem;
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			vertical-align: middle;
		}

		.wl-table tbody tr {
			transition: background-color 150ms var(--ease-out);
		}

		.wl-table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.wl-table tbody tr:last-child td {
			border-bottom: none;
		}

		.wl-table__title {
			font-weight: 600;
			color: var(--color-white);
		}

		.wl-table__muted {
			color: var(--color-grey-400);
		}

		.wl-table__status {
			display: inline-flex;
			align-items: center;
			gap: 0.4rem;
			font-size: 0.75rem;
			font-weight: 600;
			padding: 0.2rem 0.6rem;
			border-radius: var(--radius-full);
		}

		.wl-table__status-dot {
			width: 0.4rem;
			height: 0.4rem;
			border-radius: var(--radius-full);
			background: currentColor;
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
			justify-content: flex-end;
			gap: 0.4rem;
		}

		.wl-table__btn {
			width: 2rem;
			height: 2rem;
			display: inline-flex;
			align-items: center;
			justify-content: center;
			border-radius: var(--radius-md);
			border: 1px solid transparent;
			cursor: pointer;
			text-decoration: none;
			transition: all 150ms var(--ease-out);
		}

		.wl-table__btn--edit {
			background-color: rgba(59, 130, 246, 0.1);
			color: #60a5fa;
		}

		.wl-table__btn--edit:hover {
			background-color: rgba(59, 130, 246, 0.22);
		}

		.wl-table__btn--publish {
			background-color: rgba(34, 197, 94, 0.1);
			color: #22c55e;
		}

		.wl-table__btn--publish:hover {
			background-color: rgba(34, 197, 94, 0.22);
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
			padding: 0.55rem 1rem;
			font-size: 0.875rem;
		}

		.wl-admin__page-info {
			font-size: 0.875rem;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.wl-admin__skeleton-card {
			animation: none;
		}
	}
</style>
