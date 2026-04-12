<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { Popup, PopupAnalytics, PaginatedResponse } from '$lib/api/types';
	import Plus from 'phosphor-svelte/lib/Plus';
	import Trash from 'phosphor-svelte/lib/Trash';
	import PencilSimple from 'phosphor-svelte/lib/PencilSimple';
	import Copy from 'phosphor-svelte/lib/Copy';
	import CaretLeft from 'phosphor-svelte/lib/CaretLeft';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';
	import Eye from 'phosphor-svelte/lib/Eye';
	import ChartBar from 'phosphor-svelte/lib/ChartBar';
	import Lightning from 'phosphor-svelte/lib/Lightning';
	import Users from 'phosphor-svelte/lib/Users';
	import Percent from 'phosphor-svelte/lib/Percent';

	let popups = $state<Popup[]>([]);
	let analytics = $state<PopupAnalytics[]>([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	let activeCount = $derived(popups.filter((p) => p.is_active).length);
	let totalImpressions = $derived(
		analytics.reduce((sum, a) => sum + a.total_impressions, 0)
	);
	let totalSubmissions = $derived(
		analytics.reduce((sum, a) => sum + a.total_submissions, 0)
	);
	let avgConversion = $derived(
		analytics.length > 0
			? analytics.reduce((sum, a) => sum + a.conversion_rate, 0) / analytics.length
			: 0
	);

	function getAnalyticsForPopup(popupId: string): PopupAnalytics | undefined {
		return analytics.find((a) => a.popup_id === popupId);
	}

	async function load() {
		loading = true;
		try {
			const [popupRes, analyticsRes] = await Promise.all([
				api.get<PaginatedResponse<Popup>>(`/api/admin/popups?page=${page}&per_page=15`),
				api.get<PopupAnalytics[]>('/api/admin/popups/analytics')
			]);
			popups = popupRes.data;
			total = popupRes.total;
			totalPages = popupRes.total_pages;
			analytics = analyticsRes;
		} catch {
			// silently handle
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function toggleActive(popup: Popup) {
		try {
			await api.put(`/api/admin/popups/${popup.id}`, { is_active: !popup.is_active });
			await load();
		} catch {
			alert('Failed to update popup');
		}
	}

	async function duplicatePopup(popup: Popup) {
		try {
			await api.post('/api/admin/popups', {
				name: `${popup.name} (Copy)`,
				popup_type: popup.popup_type,
				trigger_type: popup.trigger_type,
				trigger_config: popup.trigger_config,
				content_json: popup.content_json,
				style_json: popup.style_json,
				targeting_rules: popup.targeting_rules,
				display_frequency: popup.display_frequency,
				frequency_config: popup.frequency_config,
				success_message: popup.success_message,
				redirect_url: popup.redirect_url,
				is_active: false,
				priority: popup.priority
			});
			await load();
		} catch {
			alert('Failed to duplicate popup');
		}
	}

	async function deletePopup(popup: Popup) {
		if (!confirm(`Delete "${popup.name}"? This cannot be undone.`)) return;
		try {
			await api.del(`/api/admin/popups/${popup.id}`);
			await load();
		} catch {
			alert('Failed to delete popup');
		}
	}

	function formatType(type: string): string {
		return type.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
	}

	function typeBadgeClass(type: string): string {
		const map: Record<string, string> = {
			modal: 'pop-badge--modal',
			slide_in: 'pop-badge--slide',
			banner: 'pop-badge--banner',
			fullscreen: 'pop-badge--full',
			floating_bar: 'pop-badge--float',
			inline: 'pop-badge--inline'
		};
		return map[type] || '';
	}
</script>

<svelte:head>
	<title>Popups - Admin - Explosive Swings</title>
</svelte:head>

<div class="pop-admin">
	<div class="pop-admin__header">
		<div>
			<h1 class="pop-admin__title">Popups</h1>
			<p class="pop-admin__count">{total} total popups</p>
		</div>
		<a href="/admin/popups/new" class="pop-admin__create">
			<Plus size={18} weight="bold" />
			Create Popup
		</a>
	</div>

	{#if loading}
		<div class="pop-admin__loading">
			<div class="pop-admin__spinner"></div>
			<p>Loading popups...</p>
		</div>
	{:else}
		<!-- Stats Row -->
		<div class="pop-stats">
			<div class="pop-stats__card">
				<div class="pop-stats__icon pop-stats__icon--active">
					<Lightning size={20} weight="bold" />
				</div>
				<div class="pop-stats__data">
					<span class="pop-stats__value">{activeCount}</span>
					<span class="pop-stats__label">Active</span>
				</div>
			</div>
			<div class="pop-stats__card">
				<div class="pop-stats__icon pop-stats__icon--impressions">
					<Eye size={20} weight="bold" />
				</div>
				<div class="pop-stats__data">
					<span class="pop-stats__value">{totalImpressions.toLocaleString()}</span>
					<span class="pop-stats__label">Impressions</span>
				</div>
			</div>
			<div class="pop-stats__card">
				<div class="pop-stats__icon pop-stats__icon--submissions">
					<Users size={20} weight="bold" />
				</div>
				<div class="pop-stats__data">
					<span class="pop-stats__value">{totalSubmissions.toLocaleString()}</span>
					<span class="pop-stats__label">Submissions</span>
				</div>
			</div>
			<div class="pop-stats__card">
				<div class="pop-stats__icon pop-stats__icon--conversion">
					<Percent size={20} weight="bold" />
				</div>
				<div class="pop-stats__data">
					<span class="pop-stats__value">{avgConversion.toFixed(1)}%</span>
					<span class="pop-stats__label">Avg Conversion</span>
				</div>
			</div>
		</div>

		{#if popups.length === 0}
			<div class="pop-admin__empty">
				<ChartBar size={40} weight="light" />
				<p>No popups created yet.</p>
				<a href="/admin/popups/new" class="pop-admin__create-link">Create your first popup</a>
			</div>
		{:else}
			<!-- Mobile: Card view -->
			<div class="pop-admin__cards">
				{#each popups as popup (popup.id)}
					{@const stats = getAnalyticsForPopup(popup.id)}
					<div class="pop-card">
						<div class="pop-card__header">
							<div class="pop-card__name-row">
								<span class="pop-card__name">{popup.name}</span>
								<span class="pop-badge {typeBadgeClass(popup.popup_type)}">
									{formatType(popup.popup_type)}
								</span>
							</div>
							<button
								class="pop-toggle"
								class:pop-toggle--on={popup.is_active}
								onclick={() => toggleActive(popup)}
								title={popup.is_active ? 'Deactivate' : 'Activate'}
							>
								<span class="pop-toggle__track">
									<span class="pop-toggle__thumb"></span>
								</span>
							</button>
						</div>
						<div class="pop-card__meta">
							<span class="pop-card__trigger">{formatType(popup.trigger_type)}</span>
							<span class="pop-card__divider">|</span>
							<span>{stats?.total_impressions.toLocaleString() ?? 0} views</span>
							<span class="pop-card__divider">|</span>
							<span>{stats?.total_submissions.toLocaleString() ?? 0} subs</span>
						</div>
						<div class="pop-card__actions">
							<a href="/admin/popups/{popup.id}" class="pop-card__btn pop-card__btn--edit">
								<PencilSimple size={16} weight="bold" />
								<span>Edit</span>
							</a>
							<button onclick={() => duplicatePopup(popup)} class="pop-card__btn pop-card__btn--dup">
								<Copy size={16} weight="bold" />
								<span>Duplicate</span>
							</button>
							<button onclick={() => deletePopup(popup)} class="pop-card__btn pop-card__btn--delete">
								<Trash size={16} weight="bold" />
								<span>Delete</span>
							</button>
						</div>
					</div>
				{/each}
			</div>

			<!-- Tablet+: Table view -->
			<div class="pop-admin__table-wrap">
				<table class="pop-table">
					<thead>
						<tr>
							<th>Name</th>
							<th>Type</th>
							<th>Trigger</th>
							<th>Active</th>
							<th>Impressions</th>
							<th>Submissions</th>
							<th>Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each popups as popup (popup.id)}
							{@const stats = getAnalyticsForPopup(popup.id)}
							<tr>
								<td class="pop-table__name">{popup.name}</td>
								<td>
									<span class="pop-badge {typeBadgeClass(popup.popup_type)}">
										{formatType(popup.popup_type)}
									</span>
								</td>
								<td class="pop-table__trigger">{formatType(popup.trigger_type)}</td>
								<td>
									<button
										class="pop-toggle"
										class:pop-toggle--on={popup.is_active}
										onclick={() => toggleActive(popup)}
										title={popup.is_active ? 'Deactivate' : 'Activate'}
									>
										<span class="pop-toggle__track">
											<span class="pop-toggle__thumb"></span>
										</span>
									</button>
								</td>
								<td class="pop-table__num">{stats?.total_impressions.toLocaleString() ?? '0'}</td>
								<td class="pop-table__num">{stats?.total_submissions.toLocaleString() ?? '0'}</td>
								<td>
									<div class="pop-table__actions">
										<a
											href="/admin/popups/{popup.id}"
											class="pop-table__btn pop-table__btn--edit"
											title="Edit"
										>
											<PencilSimple size={16} weight="bold" />
										</a>
										<button
											onclick={() => duplicatePopup(popup)}
											class="pop-table__btn pop-table__btn--dup"
											title="Duplicate"
										>
											<Copy size={16} weight="bold" />
										</button>
										<button
											onclick={() => deletePopup(popup)}
											class="pop-table__btn pop-table__btn--delete"
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
				<div class="pop-admin__pagination">
					<button
						onclick={() => { page--; load(); }}
						disabled={page <= 1}
						class="pop-admin__page-btn"
					>
						<CaretLeft size={16} weight="bold" /> Prev
					</button>
					<span class="pop-admin__page-info">Page {page} of {totalPages}</span>
					<button
						onclick={() => { page++; load(); }}
						disabled={page >= totalPages}
						class="pop-admin__page-btn"
					>
						Next <CaretRight size={16} weight="bold" />
					</button>
				</div>
			{/if}
		{/if}
	{/if}
</div>

<style>
	.pop-admin__header {
		margin-bottom: 1rem;
	}

	.pop-admin__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.pop-admin__count {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-top: 0.15rem;
	}

	.pop-admin__create {
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

	.pop-admin__create:hover {
		opacity: 0.9;
	}

	/* Loading */
	.pop-admin__loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 3rem;
		color: var(--color-grey-400);
	}

	.pop-admin__spinner {
		width: 2rem;
		height: 2rem;
		border: 2px solid rgba(255, 255, 255, 0.1);
		border-top-color: var(--color-teal);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	/* Stats */
	.pop-stats {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.pop-stats__card {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.85rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}

	.pop-stats__icon {
		width: 2.25rem;
		height: 2.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-lg);
		flex-shrink: 0;
	}

	.pop-stats__icon--active {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}

	.pop-stats__icon--impressions {
		background-color: rgba(59, 130, 246, 0.12);
		color: #3b82f6;
	}

	.pop-stats__icon--submissions {
		background-color: rgba(34, 197, 94, 0.12);
		color: #22c55e;
	}

	.pop-stats__icon--conversion {
		background-color: rgba(212, 168, 67, 0.12);
		color: var(--color-gold);
	}

	.pop-stats__data {
		display: flex;
		flex-direction: column;
	}

	.pop-stats__value {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.2;
	}

	.pop-stats__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	/* Empty */
	.pop-admin__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 3rem 1rem;
		background-color: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		color: var(--color-grey-400);
		text-align: center;
	}

	.pop-admin__create-link {
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		text-decoration: none;
	}

	/* Type Badges */
	.pop-badge {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		padding: 0.15rem 0.55rem;
		border-radius: var(--radius-full);
		white-space: nowrap;
	}

	.pop-badge--modal {
		background-color: rgba(139, 92, 246, 0.12);
		color: #a78bfa;
	}

	.pop-badge--slide {
		background-color: rgba(59, 130, 246, 0.12);
		color: #60a5fa;
	}

	.pop-badge--banner {
		background-color: rgba(212, 168, 67, 0.12);
		color: var(--color-gold-light);
	}

	.pop-badge--full {
		background-color: rgba(236, 72, 153, 0.12);
		color: #f472b6;
	}

	.pop-badge--float {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal-light);
	}

	.pop-badge--inline {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	/* Toggle */
	.pop-toggle {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		flex-shrink: 0;
	}

	.pop-toggle__track {
		display: block;
		width: 2.5rem;
		height: 1.35rem;
		border-radius: var(--radius-full);
		background-color: rgba(255, 255, 255, 0.12);
		position: relative;
		transition: background-color 200ms var(--ease-out);
	}

	.pop-toggle--on .pop-toggle__track {
		background-color: var(--color-teal);
	}

	.pop-toggle__thumb {
		display: block;
		position: absolute;
		top: 2px;
		left: 2px;
		width: 1rem;
		height: 1rem;
		background-color: var(--color-white);
		border-radius: 50%;
		transition: transform 200ms var(--ease-out);
	}

	.pop-toggle--on .pop-toggle__thumb {
		transform: translateX(1.15rem);
	}

	/* Mobile Cards */
	.pop-admin__cards {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.pop-admin__table-wrap {
		display: none;
	}

	.pop-card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.85rem 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
	}

	.pop-card__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.pop-card__name-row {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.pop-card__name {
		font-weight: var(--w-semibold);
		color: var(--color-white);
		font-size: var(--fs-sm);
	}

	.pop-card__meta {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		flex-wrap: wrap;
	}

	.pop-card__trigger {
		color: var(--color-grey-300);
	}

	.pop-card__divider {
		color: rgba(255, 255, 255, 0.1);
	}

	.pop-card__actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.25rem;
		padding-top: 0.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.pop-card__btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.3rem;
		padding: 0.5rem;
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition: background-color 200ms var(--ease-out);
	}

	.pop-card__btn--edit {
		background-color: rgba(59, 130, 246, 0.1);
		color: #3b82f6;
	}

	.pop-card__btn--edit:hover {
		background-color: rgba(59, 130, 246, 0.25);
	}

	.pop-card__btn--dup {
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
	}

	.pop-card__btn--dup:hover {
		background-color: rgba(15, 164, 175, 0.25);
	}

	.pop-card__btn--delete {
		background-color: rgba(239, 68, 68, 0.08);
		color: #ef4444;
	}

	.pop-card__btn--delete:hover {
		background-color: rgba(239, 68, 68, 0.2);
	}

	/* Pagination */
	.pop-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.pop-admin__page-btn {
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

	.pop-admin__page-btn:hover:not(:disabled) {
		border-color: var(--color-teal);
	}

	.pop-admin__page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.pop-admin__page-info {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	/* Desktop */
	@media (min-width: 768px) {
		.pop-admin__header {
			display: flex;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 1.5rem;
		}

		.pop-admin__title {
			font-size: var(--fs-2xl);
		}

		.pop-admin__create {
			margin-top: 0;
			padding: 0.65rem 1.25rem;
			font-size: var(--fs-sm);
		}

		.pop-stats {
			grid-template-columns: repeat(4, 1fr);
		}

		.pop-admin__cards {
			display: none;
		}

		.pop-admin__table-wrap {
			display: block;
			overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
		}

		.pop-table {
			width: 100%;
			border-collapse: collapse;
		}

		.pop-table th {
			text-align: left;
			font-size: var(--fs-xs);
			font-weight: var(--w-semibold);
			color: var(--color-grey-400);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			padding: 0.85rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.pop-table td {
			padding: 0.85rem 1rem;
			font-size: var(--fs-sm);
			color: var(--color-grey-300);
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		}

		.pop-table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.pop-table__name {
			font-weight: var(--w-semibold);
			color: var(--color-white);
		}

		.pop-table__trigger {
			font-size: var(--fs-xs);
			color: var(--color-grey-400);
		}

		.pop-table__num {
			font-variant-numeric: tabular-nums;
		}

		.pop-table__actions {
			display: flex;
			gap: 0.5rem;
		}

		.pop-table__btn {
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

		.pop-table__btn--edit {
			background-color: rgba(59, 130, 246, 0.1);
			color: #3b82f6;
		}

		.pop-table__btn--edit:hover {
			background-color: rgba(59, 130, 246, 0.25);
		}

		.pop-table__btn--dup {
			background-color: rgba(15, 164, 175, 0.1);
			color: var(--color-teal);
		}

		.pop-table__btn--dup:hover {
			background-color: rgba(15, 164, 175, 0.25);
		}

		.pop-table__btn--delete {
			background-color: rgba(239, 68, 68, 0.08);
			color: #ef4444;
		}

		.pop-table__btn--delete:hover {
			background-color: rgba(239, 68, 68, 0.2);
		}

		.pop-admin__pagination {
			gap: 1rem;
			margin-top: 1.5rem;
		}

		.pop-admin__page-btn {
			gap: 0.35rem;
			padding: 0.5rem 1rem;
			font-size: var(--fs-sm);
		}

		.pop-admin__page-info {
			font-size: var(--fs-sm);
		}
	}
</style>
