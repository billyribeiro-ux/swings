<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, ApiError } from '$lib/api/client';
	import type { Watchlist } from '$lib/api/types';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';

	let title = $state('');
	let weekOf = $state('');
	let videoUrl = $state('');
	let notes = $state('');
	let published = $state(false);
	let saving = $state(false);
	let error = $state('');

	async function handleSubmit(e: Event) {
		e.preventDefault();
		saving = true;
		error = '';

		try {
			const wl = await api.post<Watchlist>('/api/admin/watchlists', {
				title,
				week_of: weekOf,
				video_url: videoUrl || null,
				notes: notes || null,
				published
			});
			goto(`/admin/watchlists/${wl.id}`);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to create watchlist';
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head>
	<title>New Watchlist - Admin - Precision Options Signals</title>
</svelte:head>

<div class="new-wl">
	<a href="/admin/watchlists" class="new-wl__back">
		<ArrowLeftIcon size={18} />
		Back to Watchlists
	</a>

	<h1 class="new-wl__title">Create New Watchlist</h1>

	{#if error}
		<div class="new-wl__error">{error}</div>
	{/if}

	<form onsubmit={handleSubmit} class="new-wl__form">
		<div class="new-wl__field">
			<label for="title" class="new-wl__label">Title</label>
			<input
				id="title"
				type="text"
				bind:value={title}
				required
				class="new-wl__input"
				placeholder="e.g. Weekly Watchlist - March 31"
			/>
		</div>

		<div class="new-wl__field">
			<label for="weekOf" class="new-wl__label">Week Of</label>
			<input id="weekOf" type="date" bind:value={weekOf} required class="new-wl__input" />
		</div>

		<div class="new-wl__field">
			<label for="videoUrl" class="new-wl__label">Video URL (optional)</label>
			<input
				id="videoUrl"
				type="url"
				bind:value={videoUrl}
				class="new-wl__input"
				placeholder="https://youtube.com/..."
			/>
		</div>

		<div class="new-wl__field">
			<label for="notes" class="new-wl__label">Notes (optional)</label>
			<textarea
				id="notes"
				bind:value={notes}
				class="new-wl__textarea"
				rows="4"
				placeholder="Market context, key themes for the week..."
			></textarea>
		</div>

		<label class="new-wl__checkbox">
			<input id="watchlist-published" name="published" type="checkbox" bind:checked={published} />
			<span>Publish immediately</span>
		</label>

		<div class="new-wl__actions">
			<a href="/admin/watchlists" class="new-wl__cancel">Cancel</a>
			<button type="submit" disabled={saving} class="new-wl__submit">
				{saving ? 'Creating...' : 'Create Watchlist'}
			</button>
		</div>
	</form>
</div>

<style>
	.new-wl__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}

	.new-wl__back:hover {
		color: var(--color-white);
	}

	.new-wl__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 2rem;
	}

	.new-wl__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
	}

	.new-wl__form {
		max-width: 36rem;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 2rem;
	}

	.new-wl__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.new-wl__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.new-wl__input,
	.new-wl__textarea {
		width: 100%;
		padding: 0.7rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-base);
		font-family: inherit;
		transition: border-color 200ms var(--ease-out);
	}

	.new-wl__input:focus,
	.new-wl__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.new-wl__input::placeholder,
	.new-wl__textarea::placeholder {
		color: var(--color-grey-500);
	}

	.new-wl__textarea {
		resize: vertical;
	}

	.new-wl__checkbox {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		cursor: pointer;
	}

	.new-wl__checkbox input[type='checkbox'] {
		accent-color: var(--color-teal);
		width: 1rem;
		height: 1rem;
	}

	.new-wl__actions {
		display: flex;
		gap: 1rem;
		justify-content: flex-end;
		margin-top: 0.5rem;
	}

	.new-wl__cancel {
		padding: 0.6rem 1.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition: border-color 200ms var(--ease-out);
	}

	.new-wl__cancel:hover {
		border-color: rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}

	.new-wl__submit {
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-2xl);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}

	.new-wl__submit:hover:not(:disabled) {
		opacity: 0.9;
	}

	.new-wl__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
