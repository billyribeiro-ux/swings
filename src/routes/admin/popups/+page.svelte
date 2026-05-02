<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import type { Popup, PopupAnalytics, PaginatedResponse } from '$lib/api/types';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import CopyIcon from 'phosphor-svelte/lib/CopyIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import PercentIcon from 'phosphor-svelte/lib/PercentIcon';
	import ChatCircleDotsIcon from 'phosphor-svelte/lib/ChatCircleDotsIcon';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';
	import TableSkeleton from '$lib/components/admin/TableSkeleton.svelte';

	let popups = $state<Popup[]>([]);
	let analytics = $state<PopupAnalytics[]>([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	let activeCount = $derived(popups.filter((p) => p.is_active).length);
	let totalImpressions = $derived(analytics.reduce((sum, a) => sum + a.total_impressions, 0));
	let totalSubmissions = $derived(analytics.reduce((sum, a) => sum + a.total_submissions, 0));
	let avgConversion = $derived(
		analytics.length > 0
			? analytics.reduce((sum, a) => sum + a.conversion_rate, 0) / analytics.length
			: 0
	);

	function getAnalyticsForPopup(popupId: string): PopupAnalytics | undefined {
		return analytics.find((a) => a.popup_id === popupId);
	}

	// Backend shape (collection summary) — kept narrow so this page doesn't
	// drift from the per-popup `/{id}/analytics` shape (which keeps the
	// `total_*` field names for backwards compat).
	type PopupAnalyticsSummaryRow = {
		popup_id: string;
		popup_name: string;
		popup_type: string;
		is_active: boolean;
		impressions: number;
		closes: number;
		submits: number;
		conversion_rate: number;
	};

	async function load() {
		loading = true;
		try {
			const [popupRes, analyticsRes] = await Promise.all([
				api.get<PaginatedResponse<Popup>>(`/api/admin/popups?page=${page}&per_page=15`),
				api.get<PopupAnalyticsSummaryRow[]>('/api/admin/popups/analytics')
			]);
			popups = popupRes.data;
			total = popupRes.total;
			totalPages = popupRes.total_pages;
			// Map the collection summary onto the per-popup `PopupAnalytics`
			// shape this page was originally written against. Avoids touching
			// every consumer in the template for a one-line field rename.
			analytics = analyticsRes.map((row) => ({
				popup_id: row.popup_id,
				total_impressions: row.impressions,
				total_closes: row.closes,
				total_submissions: row.submits,
				conversion_rate: row.conversion_rate
			})) as PopupAnalytics[];
		} catch (e) {
			toast.error('Failed to load popups', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function toggleActive(popup: Popup) {
		const next = !popup.is_active;
		try {
			await api.put(`/api/admin/popups/${popup.id}`, { is_active: next });
			toast.success(next ? `Activated "${popup.name}"` : `Deactivated "${popup.name}"`);
			await load();
		} catch (e) {
			toast.error('Failed to update popup', {
				description: e instanceof Error ? e.message : undefined
			});
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
			toast.success(`Duplicated "${popup.name}"`);
			await load();
		} catch (e) {
			toast.error('Failed to duplicate popup', {
				description: e instanceof Error ? e.message : undefined
			});
		}
	}

	async function deletePopup(popup: Popup) {
		const ok = await confirmDialog({
			title: `Delete "${popup.name}"?`,
			message:
				'The popup, its targeting rules, and its analytics history will be permanently removed.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.del(`/api/admin/popups/${popup.id}`);
			toast.success(`Deleted "${popup.name}"`);
			await load();
		} catch (e) {
			toast.error('Failed to delete popup', {
				description: e instanceof Error ? e.message : undefined
			});
		}
	}

	function formatType(type: string): string {
		return type.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
	}

	function typeBadgeClass(type: string): string {
		const map: Record<string, string> = {
			modal: 'pill--modal',
			slide_in: 'pill--slide',
			banner: 'pill--banner',
			fullscreen: 'pill--full',
			floating_bar: 'pill--float',
			inline: 'pill--inline'
		};
		return map[type] || '';
	}
</script>

<svelte:head>
	<title>Popups - Admin - Precision Options Signals</title>
</svelte:head>

<div class="popups-page">
	<header class="popups-page__header">
		<div class="popups-page__title-row">
			<ChatCircleDotsIcon size={28} weight="duotone" />
			<div class="popups-page__copy">
				<h1 class="popups-page__title">Popups</h1>
				<p class="popups-page__subtitle">
					Configure modals, banners, and slide-ins. {total} total popup{total === 1
						? ''
						: 's'}.
				</p>
			</div>
		</div>
		<a href={resolve('/admin/popups/new')} class="btn btn--primary">
			<PlusIcon size={16} weight="bold" />
			<span>Create popup</span>
		</a>
	</header>

	{#if loading}
		<TableSkeleton rows={5} label="Loading popups" />
	{:else}
		<section class="stats" aria-label="Popup metrics">
			<div class="stat-card">
				<div class="stat-card__top">
					<span class="stat-card__label">Active</span>
					<span class="stat-card__icon stat-card__icon--teal">
						<LightningIcon size={16} weight="duotone" />
					</span>
				</div>
				<span class="stat-card__value">{activeCount}</span>
				<span class="stat-card__hint">Currently live</span>
			</div>
			<div class="stat-card">
				<div class="stat-card__top">
					<span class="stat-card__label">Impressions</span>
					<span class="stat-card__icon stat-card__icon--blue">
						<EyeIcon size={16} weight="duotone" />
					</span>
				</div>
				<span class="stat-card__value">{totalImpressions.toLocaleString()}</span>
				<span class="stat-card__hint">All-time views</span>
			</div>
			<div class="stat-card">
				<div class="stat-card__top">
					<span class="stat-card__label">Submissions</span>
					<span class="stat-card__icon stat-card__icon--green">
						<UsersIcon size={16} weight="duotone" />
					</span>
				</div>
				<span class="stat-card__value">{totalSubmissions.toLocaleString()}</span>
				<span class="stat-card__hint">Total conversions</span>
			</div>
			<div class="stat-card">
				<div class="stat-card__top">
					<span class="stat-card__label">Avg conversion</span>
					<span class="stat-card__icon stat-card__icon--gold">
						<PercentIcon size={16} weight="duotone" />
					</span>
				</div>
				<span class="stat-card__value">{avgConversion.toFixed(1)}%</span>
				<span class="stat-card__hint">Across all popups</span>
			</div>
		</section>

		{#if popups.length === 0}
			<div class="empty" role="status">
				<div class="empty__icon" aria-hidden="true">
					<ChatCircleDotsIcon size={28} weight="duotone" />
				</div>
				<p class="empty__title">No popups yet</p>
				<p class="empty__sub">
					Modals, slide-ins, banners — capture attention with a targeted prompt. Build
					your first one and we'll start tracking impressions and conversions.
				</p>
				<a href={resolve('/admin/popups/new')} class="btn btn--primary">
					<PlusIcon size={16} weight="bold" />
					<span>Create first popup</span>
				</a>
			</div>
		{:else}
			<!-- Mobile: Card view -->
			<div class="cards">
				{#each popups as popup (popup.id)}
					{@const stats = getAnalyticsForPopup(popup.id)}
					<article class="popup-card">
						<div class="popup-card__head">
							<div class="popup-card__name-row">
								<span class="popup-card__name">{popup.name}</span>
								<span class="pill {typeBadgeClass(popup.popup_type)}">
									{formatType(popup.popup_type)}
								</span>
							</div>
							<Tooltip
								label={popup.is_active ? 'Deactivate popup' : 'Activate popup'}
							>
								<button
									class="toggle"
									class:toggle--on={popup.is_active}
									onclick={() => toggleActive(popup)}
									aria-label={popup.is_active ? 'Deactivate' : 'Activate'}
								>
									<span class="toggle__track">
										<span class="toggle__thumb"></span>
									</span>
								</button>
							</Tooltip>
						</div>
						<div class="popup-card__meta">
							<span class="popup-card__trigger">{formatType(popup.trigger_type)}</span
							>
							<span class="popup-card__divider" aria-hidden="true">·</span>
							<span>{stats?.total_impressions.toLocaleString() ?? 0} views</span>
							<span class="popup-card__divider" aria-hidden="true">·</span>
							<span>{stats?.total_submissions.toLocaleString() ?? 0} subs</span>
						</div>
						<div class="popup-card__actions">
							<a
								href={resolve('/admin/popups/[id]', { id: popup.id })}
								class="action-btn"
							>
								<PencilSimpleIcon size={14} weight="bold" />
								<span>Edit</span>
							</a>
							<button onclick={() => duplicatePopup(popup)} class="action-btn">
								<CopyIcon size={14} weight="bold" />
								<span>Duplicate</span>
							</button>
							<button
								onclick={() => deletePopup(popup)}
								class="action-btn action-btn--destructive"
							>
								<TrashIcon size={14} weight="bold" />
								<span>Delete</span>
							</button>
						</div>
					</article>
				{/each}
			</div>

			<!-- Tablet+: Table view -->
			<section class="card table-card">
				<div class="table-wrap">
					<table class="table">
						<thead>
							<tr>
								<th scope="col">Name</th>
								<th scope="col">Type</th>
								<th scope="col">Trigger</th>
								<th scope="col">Active</th>
								<th scope="col" class="table__num-th">Impressions</th>
								<th scope="col" class="table__num-th">Submissions</th>
								<th scope="col" class="table__actions-th">Actions</th>
							</tr>
						</thead>
						<tbody>
							{#each popups as popup (popup.id)}
								{@const stats = getAnalyticsForPopup(popup.id)}
								<tr>
									<td class="table__name">{popup.name}</td>
									<td>
										<span class="pill {typeBadgeClass(popup.popup_type)}">
											{formatType(popup.popup_type)}
										</span>
									</td>
									<td class="table__trigger">{formatType(popup.trigger_type)}</td>
									<td>
										<Tooltip
											label={popup.is_active
												? 'Deactivate popup'
												: 'Activate popup'}
										>
											<button
												class="toggle"
												class:toggle--on={popup.is_active}
												onclick={() => toggleActive(popup)}
												aria-label={popup.is_active
													? 'Deactivate'
													: 'Activate'}
											>
												<span class="toggle__track">
													<span class="toggle__thumb"></span>
												</span>
											</button>
										</Tooltip>
									</td>
									<td class="table__num"
										>{stats?.total_impressions.toLocaleString() ?? '0'}</td
									>
									<td class="table__num"
										>{stats?.total_submissions.toLocaleString() ?? '0'}</td
									>
									<td>
										<div class="row-actions">
											<Tooltip label="Edit popup">
												<a
													href={resolve('/admin/popups/[id]', {
														id: popup.id
													})}
													class="icon-btn"
													aria-label="Edit"
												>
													<PencilSimpleIcon size={14} weight="bold" />
												</a>
											</Tooltip>
											<Tooltip label="Duplicate popup">
												<button
													onclick={() => duplicatePopup(popup)}
													class="icon-btn"
													aria-label="Duplicate"
												>
													<CopyIcon size={14} weight="bold" />
												</button>
											</Tooltip>
											<Tooltip label="Delete popup">
												<button
													onclick={() => deletePopup(popup)}
													class="icon-btn icon-btn--destructive"
													aria-label="Delete"
												>
													<TrashIcon size={14} weight="bold" />
												</button>
											</Tooltip>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</section>

			{#if totalPages > 1}
				<div class="pager">
					<button
						onclick={() => {
							page--;
							load();
						}}
						disabled={page <= 1}
						class="btn btn--secondary"
					>
						<CaretLeftIcon size={16} weight="bold" />
						<span>Prev</span>
					</button>
					<span class="pager__info">Page {page} of {totalPages}</span>
					<button
						onclick={() => {
							page++;
							load();
						}}
						disabled={page >= totalPages}
						class="btn btn--secondary"
					>
						<span>Next</span>
						<CaretRightIcon size={16} weight="bold" />
					</button>
				</div>
			{/if}
		{/if}
	{/if}
</div>

<style>
	.popups-page {
		max-width: 80rem;
		padding: 0 0 3rem;
	}
	.popups-page__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	.popups-page__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.popups-page__copy {
		min-width: 0;
	}
	.popups-page__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.2;
	}
	.popups-page__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 3rem;
		padding: 0 1.25rem;
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		font-weight: 600;
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		text-decoration: none;
		font-family: inherit;
		align-self: flex-start;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms,
			box-shadow 150ms,
			transform 150ms;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
	}
	.btn--primary:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55);
	}
	.btn--secondary {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.1);
		color: var(--color-grey-200);
	}
	.btn--secondary:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}
	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.stats {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}
	.stat-card {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.stat-card__top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.stat-card__label {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.stat-card__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.85rem;
		height: 1.85rem;
		border-radius: var(--radius-md);
	}
	.stat-card__icon--teal {
		background: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}
	.stat-card__icon--blue {
		background: rgba(59, 130, 246, 0.12);
		color: #60a5fa;
	}
	.stat-card__icon--green {
		background: rgba(34, 197, 94, 0.12);
		color: #4ade80;
	}
	.stat-card__icon--gold {
		background: rgba(212, 168, 67, 0.12);
		color: var(--color-gold);
	}
	.stat-card__value {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		font-variant-numeric: tabular-nums;
		letter-spacing: -0.01em;
		line-height: 1.15;
	}
	.stat-card__hint {
		font-size: 0.75rem;
		color: var(--color-grey-400);
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.65rem;
		padding: clamp(2.5rem, 6vw, 3.5rem) 1.5rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-500);
		text-align: center;
	}
	.empty__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 3.5rem;
		height: 3.5rem;
		border-radius: var(--radius-full);
		background: rgba(15, 164, 175, 0.1);
		color: var(--color-teal-light);
	}
	.empty__title {
		margin: 0.25rem 0 0;
		font-family: var(--font-heading);
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--color-white);
		letter-spacing: -0.01em;
	}
	.empty__sub {
		margin: 0 0 0.625rem;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 38ch;
		line-height: 1.55;
	}

	.pill {
		display: inline-flex;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}
	.pill--modal {
		background: rgba(139, 92, 246, 0.12);
		color: #a78bfa;
	}
	.pill--slide {
		background: rgba(59, 130, 246, 0.12);
		color: #60a5fa;
	}
	.pill--banner {
		background: rgba(212, 168, 67, 0.12);
		color: var(--color-gold-light);
	}
	.pill--full {
		background: rgba(236, 72, 153, 0.12);
		color: #f472b6;
	}
	.pill--float {
		background: rgba(15, 164, 175, 0.12);
		color: var(--color-teal-light);
	}
	.pill--inline {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	.toggle {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		flex-shrink: 0;
	}
	.toggle__track {
		display: block;
		width: 2.5rem;
		height: 1.35rem;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.12);
		position: relative;
		transition: background-color 200ms var(--ease-out);
	}
	.toggle--on .toggle__track {
		background: var(--color-teal);
	}
	.toggle__thumb {
		display: block;
		position: absolute;
		top: 2px;
		left: 2px;
		width: 1rem;
		height: 1rem;
		background: var(--color-white);
		border-radius: 50%;
		transition: transform 200ms var(--ease-out);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
	}
	.toggle--on .toggle__thumb {
		transform: translateX(1.15rem);
	}

	.cards {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.popup-card {
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
		padding: 1rem 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.popup-card__head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}
	.popup-card__name-row {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		min-width: 0;
	}
	.popup-card__name {
		font-weight: 600;
		color: var(--color-white);
		font-size: 0.875rem;
	}
	.popup-card__meta {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.75rem;
		color: var(--color-grey-400);
		flex-wrap: wrap;
	}
	.popup-card__trigger {
		color: var(--color-grey-300);
	}
	.popup-card__divider {
		color: rgba(255, 255, 255, 0.2);
	}
	.popup-card__actions {
		display: flex;
		gap: 0.4rem;
		padding-top: 0.65rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		flex-wrap: wrap;
	}

	.card {
		display: none;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.table-card {
		overflow: hidden;
	}
	.table-wrap {
		overflow-x: auto;
	}
	.table {
		width: 100%;
		border-collapse: collapse;
	}
	.table th {
		text-align: left;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.75rem 1rem;
		background: rgba(255, 255, 255, 0.02);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		white-space: nowrap;
	}
	.table__num-th {
		text-align: right;
	}
	.table__actions-th {
		text-align: right;
	}
	.table td {
		padding: 0.875rem 1rem;
		font-size: 0.875rem;
		color: var(--color-grey-200);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		vertical-align: middle;
	}
	.table tbody tr:hover td {
		background: rgba(255, 255, 255, 0.02);
	}
	.table tbody tr:last-child td {
		border-bottom: none;
	}
	.table__name {
		font-weight: 600;
		color: var(--color-white);
	}
	.table__trigger {
		font-size: 0.75rem;
		color: var(--color-grey-400);
	}
	.table__num {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	.row-actions {
		display: inline-flex;
		gap: 0.4rem;
		justify-content: flex-end;
	}
	.icon-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.05);
		color: var(--color-grey-200);
		cursor: pointer;
		text-decoration: none;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms;
	}
	.icon-btn:hover {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}
	.icon-btn--destructive {
		background: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}
	.icon-btn--destructive:hover {
		background: rgba(239, 68, 68, 0.18);
		border-color: rgba(239, 68, 68, 0.4);
	}

	@media (max-width: 767px) {
		.icon-btn {
			min-width: 2.75rem;
			min-height: 2.75rem;
		}
	}

	.action-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		min-height: 2.5rem;
		padding: 0 0.65rem;
		font-size: 0.75rem;
		font-weight: 600;
		border: 1px solid rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.05);
		color: var(--color-grey-200);
		border-radius: var(--radius-2xl);
		text-decoration: none;
		cursor: pointer;
		font-family: inherit;
		flex: 1;
		justify-content: center;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms;
	}
	.action-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}
	.action-btn--destructive {
		background: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}
	.action-btn--destructive:hover {
		background: rgba(239, 68, 68, 0.18);
	}

	.pager {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1.25rem;
		flex-wrap: wrap;
	}
	.pager__info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
	}

	@media (min-width: 768px) {
		.popups-page__header {
			flex-direction: row;
			align-items: flex-start;
			justify-content: space-between;
		}
		.cards {
			display: none;
		}
		.card {
			display: block;
		}
	}
	@media (min-width: 1024px) {
		.stats {
			grid-template-columns: repeat(4, minmax(0, 1fr));
		}
	}
</style>
