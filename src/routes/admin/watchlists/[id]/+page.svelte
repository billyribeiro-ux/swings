<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { WatchlistWithAlerts, WatchlistAlert } from '$lib/api/types';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	let watchlist = $state<WatchlistWithAlerts | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');
	let successMsg = $state('');

	// Editable fields
	let title = $state('');
	let weekOf = $state('');
	let videoUrl = $state('');
	let notes = $state('');
	let published = $state(false);

	// New alert form
	let showAlertForm = $state(false);
	let alertTicker = $state('');
	let alertDirection = $state<'bullish' | 'bearish'>('bullish');
	let alertEntry = $state('');
	let alertInvalidation = $state('');
	let alertProfitZones = $state('');
	let alertNotes = $state('');
	let alertChartUrl = $state('');
	let alertSaving = $state(false);

	onMount(async () => {
		try {
			const id = page.params.id;
			watchlist = await api.get<WatchlistWithAlerts>(`/api/admin/watchlists/${id}`);
			title = watchlist.title;
			weekOf = watchlist.week_of;
			videoUrl = watchlist.video_url ?? '';
			notes = watchlist.notes ?? '';
			published = watchlist.published;
		} catch {
			error = 'Watchlist not found';
		} finally {
			loading = false;
		}
	});

	async function saveWatchlist(e: Event) {
		e.preventDefault();
		saving = true;
		error = '';
		successMsg = '';

		try {
			await api.put(`/api/admin/watchlists/${page.params.id}`, {
				title,
				week_of: weekOf,
				video_url: videoUrl || null,
				notes: notes || null,
				published
			});
			successMsg = 'Watchlist saved!';
			setTimeout(() => (successMsg = ''), 3000);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to save';
		} finally {
			saving = false;
		}
	}

	async function addAlert(e: Event) {
		e.preventDefault();
		alertSaving = true;

		try {
			const newAlert = await api.post<WatchlistAlert>(
				`/api/admin/watchlists/${page.params.id}/alerts`,
				{
					ticker: alertTicker.toUpperCase(),
					direction: alertDirection,
					entry_zone: alertEntry,
					invalidation: alertInvalidation,
					profit_zones: alertProfitZones
						.split(',')
						.map((s: string) => s.trim())
						.filter(Boolean),
					notes: alertNotes || null,
					chart_url: alertChartUrl || null
				}
			);

			if (watchlist) {
				watchlist = { ...watchlist, alerts: [...watchlist.alerts, newAlert] };
			}

			// Reset form
			alertTicker = '';
			alertDirection = 'bullish';
			alertEntry = '';
			alertInvalidation = '';
			alertProfitZones = '';
			alertNotes = '';
			alertChartUrl = '';
			showAlertForm = false;
		} catch (err) {
			alert(err instanceof ApiError ? err.message : 'Failed to add alert');
		} finally {
			alertSaving = false;
		}
	}

	async function deleteAlert(alertId: string) {
		const ok = await confirmDialog({
			title: 'Delete this alert?',
			message: 'The alert will be removed from this watchlist immediately.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.del(`/api/admin/alerts/${alertId}`);
			if (watchlist) {
				watchlist = { ...watchlist, alerts: watchlist.alerts.filter((a) => a.id !== alertId) };
			}
		} catch {
			alert('Failed to delete alert');
		}
	}
</script>

<svelte:head>
	<title>Edit Watchlist - Admin - Precision Options Signals</title>
</svelte:head>

<div class="edit-wl">
	<a href="/admin/watchlists" class="edit-wl__back">
		<ArrowLeftIcon size={18} />
		Back to Watchlists
	</a>

	{#if loading}
		<p class="edit-wl__loading">Loading...</p>
	{:else if error && !watchlist}
		<p class="edit-wl__error-msg">{error}</p>
	{:else if watchlist}
		<!-- Watchlist Details Form -->
		<form onsubmit={saveWatchlist} class="edit-wl__form">
			<h2 class="edit-wl__title">Edit Watchlist</h2>

			{#if error}
				<div class="edit-wl__error">{error}</div>
			{/if}
			{#if successMsg}
				<div class="edit-wl__success">{successMsg}</div>
			{/if}

			<div class="edit-wl__grid">
				<div class="edit-wl__field">
					<label for="title" class="edit-wl__label">Title</label>
					<input id="title" type="text" bind:value={title} required class="edit-wl__input" />
				</div>

				<div class="edit-wl__field">
					<label for="weekOf" class="edit-wl__label">Week Of</label>
					<input id="weekOf" type="date" bind:value={weekOf} required class="edit-wl__input" />
				</div>

				<div class="edit-wl__field edit-wl__field--full">
					<label for="videoUrl" class="edit-wl__label">Video URL</label>
					<input
						id="videoUrl"
						type="url"
						bind:value={videoUrl}
						class="edit-wl__input"
						placeholder="https://..."
					/>
				</div>

				<div class="edit-wl__field edit-wl__field--full">
					<label for="notes" class="edit-wl__label">Notes</label>
					<textarea id="notes" bind:value={notes} class="edit-wl__textarea" rows="3"></textarea>
				</div>
			</div>

			<div class="edit-wl__row">
				<label class="edit-wl__checkbox">
					<input
						id="watchlist-published"
						name="published"
						type="checkbox"
						bind:checked={published}
					/>
					<span>Published</span>
				</label>

				<button type="submit" disabled={saving} class="edit-wl__save">
					<FloppyDiskIcon size={16} weight="bold" />
					{saving ? 'Saving...' : 'Save Changes'}
				</button>
			</div>
		</form>

		<!-- Alerts Section -->
		<section class="edit-wl__alerts">
			<div class="edit-wl__alerts-header">
				<h2 class="edit-wl__alerts-title">Alerts ({watchlist.alerts.length})</h2>
				<button onclick={() => (showAlertForm = !showAlertForm)} class="edit-wl__add-alert">
					<PlusIcon size={16} weight="bold" />
					{showAlertForm ? 'Cancel' : 'Add Alert'}
				</button>
			</div>

			{#if showAlertForm}
				<form onsubmit={addAlert} class="alert-form">
					<div class="alert-form__grid">
						<div class="alert-form__field">
							<label for="alertTicker" class="alert-form__label">Ticker</label>
							<input
								id="alertTicker"
								type="text"
								bind:value={alertTicker}
								required
								class="alert-form__input"
								placeholder="AAPL"
							/>
						</div>
						<div class="alert-form__field">
							<label for="alertDirection" class="alert-form__label">Direction</label>
							<select id="alertDirection" bind:value={alertDirection} class="alert-form__input">
								<option value="bullish">Bullish</option>
								<option value="bearish">Bearish</option>
							</select>
						</div>
						<div class="alert-form__field">
							<label for="alertEntry" class="alert-form__label">Entry Zone</label>
							<input
								id="alertEntry"
								type="text"
								bind:value={alertEntry}
								required
								class="alert-form__input"
								placeholder="182.50 - 183.20"
							/>
						</div>
						<div class="alert-form__field">
							<label for="alertInvalidation" class="alert-form__label">Invalidation</label>
							<input
								id="alertInvalidation"
								type="text"
								bind:value={alertInvalidation}
								required
								class="alert-form__input"
								placeholder="Below 181.90"
							/>
						</div>
						<div class="alert-form__field alert-form__field--full">
							<label for="alertProfitZones" class="alert-form__label"
								>Profit Zones (comma-separated)</label
							>
							<input
								id="alertProfitZones"
								type="text"
								bind:value={alertProfitZones}
								required
								class="alert-form__input"
								placeholder="185.20, 186.40, 188.00"
							/>
						</div>
						<div class="alert-form__field alert-form__field--full">
							<label for="alertNotes" class="alert-form__label">Notes (optional)</label>
							<input
								id="alertNotes"
								type="text"
								bind:value={alertNotes}
								class="alert-form__input"
								placeholder="Watching for breakout above 20-day MA..."
							/>
						</div>
						<div class="alert-form__field alert-form__field--full">
							<label for="alertChartUrl" class="alert-form__label">Chart URL (optional)</label>
							<input
								id="alertChartUrl"
								type="url"
								bind:value={alertChartUrl}
								class="alert-form__input"
								placeholder="https://..."
							/>
						</div>
					</div>
					<button type="submit" disabled={alertSaving} class="alert-form__submit">
						{alertSaving ? 'Adding...' : 'Add Alert'}
					</button>
				</form>
			{/if}

			{#if watchlist.alerts.length === 0}
				<p class="edit-wl__no-alerts">No alerts yet. Add your first alert above.</p>
			{:else}
				<div class="edit-wl__alert-list">
					{#each watchlist.alerts as alert (alert.id)}
						<div class="alert-row">
							<div class="alert-row__main">
								<span
									class={[
										'alert-row__dir',
										alert.direction === 'bullish' ? 'alert-row__dir--bull' : 'alert-row__dir--bear'
									]}
								>
									{alert.direction === 'bullish' ? '▲' : '▼'}
								</span>
								<h4 class="alert-row__ticker">{alert.ticker}</h4>
								<span class="alert-row__entry">Entry: {alert.entry_zone}</span>
								<span class="alert-row__inv">Inv: {alert.invalidation}</span>
								<span class="alert-row__targets">TP: {alert.profit_zones.join(', ')}</span>
							</div>
							<button
								onclick={() => deleteAlert(alert.id)}
								class="alert-row__delete"
								title="Delete alert"
							>
								<TrashIcon size={16} weight="bold" />
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</div>

<style>
	.edit-wl__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}

	.edit-wl__back:hover {
		color: var(--color-white);
	}

	.edit-wl__loading,
	.edit-wl__error-msg {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400);
	}

	.edit-wl__form {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.5rem;
		margin-bottom: 2rem;
	}

	.edit-wl__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 1.25rem;
	}

	.edit-wl__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.6rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}

	.edit-wl__success {
		background-color: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.3);
		color: #86efac;
		padding: 0.6rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}

	.edit-wl__grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-bottom: 1.25rem;
	}

	.edit-wl__field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.edit-wl__field--full {
		grid-column: 1 / -1;
	}

	.edit-wl__label {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
	}

	.edit-wl__input,
	.edit-wl__textarea {
		padding: 0.6rem 0.8rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
		transition: border-color 200ms var(--ease-out);
	}

	.edit-wl__input:focus,
	.edit-wl__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.edit-wl__textarea {
		resize: vertical;
	}

	.edit-wl__row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.edit-wl__checkbox {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		cursor: pointer;
	}

	.edit-wl__checkbox input {
		accent-color: var(--color-teal);
	}

	.edit-wl__save {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.55rem 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-2xl);
		cursor: pointer;
	}

	.edit-wl__save:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Alerts section */
	.edit-wl__alerts-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}

	.edit-wl__alerts-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.edit-wl__add-alert {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.5rem 1rem;
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-2xl);
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}

	.edit-wl__add-alert:hover {
		background-color: rgba(15, 164, 175, 0.2);
	}

	.alert-form {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.25rem;
		margin-bottom: 1.5rem;
	}

	.alert-form__grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.alert-form__field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.alert-form__field--full {
		grid-column: 1 / -1;
	}

	.alert-form__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.alert-form__input {
		padding: 0.55rem 0.75rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
	}

	.alert-form__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.alert-form__submit {
		padding: 0.55rem 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-2xl);
		cursor: pointer;
	}

	.alert-form__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.edit-wl__no-alerts {
		color: var(--color-grey-400);
		text-align: center;
		padding: 2rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border-radius: var(--radius-2xl);
		border: 1px dashed rgba(255, 255, 255, 0.1);
	}

	.edit-wl__alert-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.alert-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.85rem 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
	}

	.alert-row__main {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.alert-row__dir {
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
	}

	.alert-row__dir--bull {
		color: #22c55e;
	}
	.alert-row__dir--bear {
		color: #ef4444;
	}

	.alert-row__ticker {
		font-size: var(--fs-base);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.alert-row__entry,
	.alert-row__inv,
	.alert-row__targets {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.alert-row__delete {
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-2xl);
		background-color: rgba(239, 68, 68, 0.08);
		color: #ef4444;
		border: none;
		cursor: pointer;
		flex-shrink: 0;
	}

	.alert-row__delete:hover {
		background-color: rgba(239, 68, 68, 0.2);
	}

	@media (max-width: 640px) {
		.edit-wl__grid,
		.alert-form__grid {
			grid-template-columns: 1fr;
		}
	}
</style>
