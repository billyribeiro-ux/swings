<script lang="ts">
	import { onMount } from 'svelte';
	import GlobeIcon from 'phosphor-svelte/lib/GlobeIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import ToggleLeftIcon from 'phosphor-svelte/lib/ToggleLeftIcon';
	import ToggleRightIcon from 'phosphor-svelte/lib/ToggleRightIcon';
	import ArrowClockwiseIcon from 'phosphor-svelte/lib/ArrowClockwiseIcon';
	import { ApiError } from '$lib/api/client';
	import { ipAllowlist, type AllowlistEntry } from '$lib/api/admin-security';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	let entries = $state<AllowlistEntry[]>([]);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let newCidr = $state('');
	let newLabel = $state('');
	let creating = $state(false);

	async function refresh() {
		loading = true;
		error = '';
		try {
			const res = await ipAllowlist.list();
			entries = res.data;
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load entries';
		} finally {
			loading = false;
		}
	}

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function createEntry(e: Event) {
		e.preventDefault();
		if (!newCidr.trim() || !newLabel.trim()) return;
		creating = true;
		error = '';
		try {
			await ipAllowlist.create({ cidr: newCidr.trim(), label: newLabel.trim() });
			newCidr = '';
			newLabel = '';
			flash('Entry added');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to create entry';
		} finally {
			creating = false;
		}
	}

	async function toggle(entry: AllowlistEntry) {
		try {
			await ipAllowlist.toggle(entry.id, !entry.is_active);
			flash(entry.is_active ? 'Entry disabled' : 'Entry enabled');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to toggle entry';
		}
	}

	async function remove(entry: AllowlistEntry) {
		const ok = await confirmDialog({
			title: `Remove ${entry.cidr}?`,
			message: `${entry.label} will be removed from the IP allowlist. If this was the last active entry, the allowlist will fail open.`,
			confirmLabel: 'Remove',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await ipAllowlist.remove(entry.id);
			flash('Entry removed');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to remove entry';
		}
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>IP allowlist · Security · Admin</title>
</svelte:head>

<div class="page" data-testid="security-ip-allowlist">
	<header class="page__header">
		<a href="/admin/security" class="page__back">← Security</a>
		<div class="page__title-row">
			<GlobeIcon size={26} weight="duotone" />
			<h1 class="page__title">IP allowlist</h1>
		</div>
		<p class="page__subtitle">
			CIDR rules that gate <code>/api/admin/*</code>. Disable an entry to keep the row for
			audit trail without enforcing it.
		</p>
	</header>

	{#if toast}
		<div class="toast">{toast}</div>
	{/if}
	{#if error}
		<div class="error" role="alert" data-testid="ip-allowlist-error">{error}</div>
	{/if}

	<section class="card">
		<h2 class="card__title">Add entry</h2>
		<form class="card__body form" onsubmit={createEntry}>
			<div class="field">
				<label class="field__label" for="ip-cidr">CIDR</label>
				<input
					id="ip-cidr"
					data-testid="ip-cidr-input"
					class="field__input"
					placeholder="203.0.113.4/32"
					bind:value={newCidr}
					required
				/>
			</div>
			<div class="field">
				<label class="field__label" for="ip-label">Label</label>
				<input
					id="ip-label"
					data-testid="ip-label-input"
					class="field__input"
					placeholder="Office static IP"
					bind:value={newLabel}
					required
				/>
			</div>
			<button
				class="btn btn--primary"
				type="submit"
				data-testid="ip-allowlist-create"
				disabled={creating}
			>
				<PlusIcon size={16} weight="bold" />
				{creating ? 'Adding…' : 'Add'}
			</button>
		</form>
	</section>

	<section class="card">
		<header class="card__heading">
			<h2 class="card__title">Active rules</h2>
			<button class="btn btn--ghost" onclick={refresh} aria-label="Refresh">
				<ArrowClockwiseIcon size={16} weight="bold" />
				Refresh
			</button>
		</header>

		{#if loading}
			<p class="muted">Loading…</p>
		{:else if entries.length === 0}
			<p class="muted">No allowlist entries yet — the middleware is currently a no-op.</p>
		{:else}
			<table class="table" data-testid="ip-allowlist-table">
				<thead>
					<tr>
						<th>CIDR</th>
						<th>Label</th>
						<th>Status</th>
						<th>Created</th>
						<th>Updated</th>
						<th aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each entries as entry (entry.id)}
						<tr>
							<td><code>{entry.cidr}</code></td>
							<td>{entry.label}</td>
							<td>
								<span
									class="badge"
									class:badge--ok={entry.is_active}
									class:badge--off={!entry.is_active}
								>
									{entry.is_active ? 'Active' : 'Disabled'}
								</span>
							</td>
							<td>{new Date(entry.created_at).toLocaleDateString()}</td>
							<td>{new Date(entry.updated_at).toLocaleString()}</td>
							<td class="row-actions">
								<button
									class="btn btn--ghost"
									onclick={() => toggle(entry)}
									aria-label={entry.is_active ? 'Disable entry' : 'Enable entry'}
								>
									{#if entry.is_active}
										<ToggleRightIcon size={18} weight="duotone" />
									{:else}
										<ToggleLeftIcon size={18} weight="duotone" />
									{/if}
								</button>
								<button
									class="btn btn--danger"
									onclick={() => remove(entry)}
									aria-label="Remove entry"
								>
									<TrashIcon size={16} weight="bold" />
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</section>
</div>

<style>
	.page {
		max-width: 960px;
	}
	.page__header {
		margin-bottom: var(--space-6);
	}
	.page__back {
		display: inline-block;
		margin-bottom: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-teal);
		text-decoration: none;
	}
	.page__back:hover {
		text-decoration: underline;
	}
	.page__title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-white);
	}
	.page__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		margin: 0;
	}
	.page__subtitle {
		margin-top: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		max-width: 60ch;
	}
	.toast {
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-5);
		margin-bottom: var(--space-5);
	}
	.card__heading {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}
	.card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0 0 var(--space-3);
	}
	.card__body {
		padding: 0;
	}
	.form {
		display: grid;
		grid-template-columns: 2fr 2fr auto;
		gap: var(--space-3);
		align-items: end;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
		font-weight: var(--w-medium);
	}
	.field__input {
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn--primary {
		background: var(--color-teal);
		color: var(--color-white);
	}
	.btn--primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn--ghost {
		border-color: rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
	}
	.btn--ghost:hover {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}
	.btn--danger {
		background: rgba(239, 68, 68, 0.1);
		color: #fca5a5;
		border-color: rgba(239, 68, 68, 0.25);
	}
	.btn--danger:hover {
		background: rgba(239, 68, 68, 0.18);
	}
	.table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.table th {
		text-align: left;
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		padding: var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}
	.table td {
		padding: var(--space-3) var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}
	.row-actions {
		display: flex;
		gap: var(--space-2);
		justify-content: flex-end;
	}
	.badge {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
	}
	.badge--ok {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}
	.badge--off {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
</style>
