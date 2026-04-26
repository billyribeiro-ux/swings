<!--
  Phase 2.1 — Notification suppression list. Manually suppressed addresses
  cannot receive any sends regardless of template / channel preferences.
  Adds and removes are audit-row backed.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import { ApiError } from '$lib/api/client';
	import {
		suppression,
		type AddSuppressionBody,
		type PaginatedSuppressionResponse,
		type Suppression,
		type SuppressionListQuery
	} from '$lib/api/admin-notifications';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import { toast } from '$lib/stores/toast.svelte';

	let envelope = $state<PaginatedSuppressionResponse | null>(null);
	let loading = $state(true);
	let error = $state('');

	let filters = $state<SuppressionListQuery>({ page: 1, per_page: 50 });

	let drawerOpen = $state(false);
	let formEmail = $state('');
	let formReason = $state('');
	let formBusy = $state(false);

	let removingEmail = $state<string | null>(null);

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await suppression.list(filters);
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load list';
		} finally {
			loading = false;
		}
	}

	function openAdd() {
		formEmail = '';
		formReason = '';
		drawerOpen = true;
	}

	async function addSuppression() {
		if (!formEmail.trim() || !formReason.trim()) {
			toast.warning('Email and reason are both required');
			return;
		}
		formBusy = true;
		error = '';
		try {
			const body: AddSuppressionBody = {
				email: formEmail.trim(),
				reason: formReason.trim()
			};
			await suppression.add(body);
			toast.success(`Suppressed ${body.email}`);
			drawerOpen = false;
			await refresh();
		} catch (e) {
			toast.error('Suppression failed', {
				description: e instanceof ApiError ? `${e.status}: ${e.message}` : undefined
			});
		} finally {
			formBusy = false;
		}
	}

	async function removeOne(row: Suppression) {
		const ok = await confirmDialog({
			title: `Remove ${row.email} from the suppression list?`,
			message:
				'After removal, this address can receive notifications again subject to channel preferences.',
			confirmLabel: 'Remove',
			variant: 'warning'
		});
		if (!ok) return;
		removingEmail = row.email;
		error = '';
		try {
			await suppression.remove(row.email);
			toast.success(`Removed ${row.email}`);
			await refresh();
		} catch (e) {
			toast.error('Remove failed', {
				description: e instanceof ApiError ? `${e.status}: ${e.message}` : undefined
			});
		} finally {
			removingEmail = null;
		}
	}

	function nextPage() {
		if (!envelope) return;
		if ((envelope.page ?? 1) >= (envelope.total_pages ?? 1)) return;
		filters.page = (filters.page ?? 1) + 1;
		void refresh();
	}
	function prevPage() {
		filters.page = Math.max(1, (filters.page ?? 1) - 1);
		void refresh();
	}

	const summaryRange = $derived.by(() => {
		if (!envelope) return '';
		const start = ((envelope.page ?? 1) - 1) * (envelope.per_page ?? 50) + 1;
		const end = start + envelope.data.length - 1;
		return `${start}–${end} of ${envelope.total}`;
	});

	onMount(refresh);
</script>

<svelte:head>
	<title>Suppression list · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-notifications-suppression">
	<header class="page__header">
		<div class="page__title-row">
			<XCircleIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Operations / Notifications</span>
				<h1 class="page__title">Suppression list</h1>
				<p class="page__subtitle">
					Addresses on this list never receive sends, regardless of template or member
					preference. Bounces and complaints feed in automatically; manual entries below
					cover ops-driven blocks.
				</p>
			</div>
		</div>
		<div class="page__actions">
			<button class="btn btn--primary" type="button" onclick={openAdd}>
				<PlusIcon size={16} weight="bold" />
				<span>Add suppression</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={() => void refresh()}>
				<ArrowsClockwiseIcon size={16} weight="bold" />
				<span>Refresh</span>
			</button>
		</div>
	</header>

	{#if error}
		<div class="error" role="alert">
			<WarningIcon size={16} weight="fill" />
			<span>{error}</span>
		</div>
	{/if}

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading suppressed addresses…</span>
		</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty">
			<XCircleIcon size={48} weight="duotone" />
			<p class="empty__title">No suppressed addresses</p>
			<p class="empty__sub">A clean list is a healthy list.</p>
			<button class="btn btn--primary" type="button" onclick={openAdd}>
				<PlusIcon size={16} weight="bold" />
				<span>Add suppression</span>
			</button>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Email</th>
							<th scope="col">Reason</th>
							<th scope="col">Suppressed at</th>
							<th scope="col" class="table__actions-th" aria-label="Action"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.data as row (row.email)}
							<tr>
								<td><code>{row.email}</code></td>
								<td>{row.reason}</td>
								<td class="ts">{new Date(row.suppressed_at).toLocaleString()}</td>
								<td class="row-actions">
									<button
										class="btn btn--danger btn--small"
										type="button"
										onclick={() => removeOne(row)}
										disabled={removingEmail === row.email}
										aria-label="Remove"
									>
										<TrashIcon size={14} weight="bold" />
										<span
											>{removingEmail === row.email
												? 'Removing…'
												: 'Remove'}</span
										>
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>

		<div class="pager">
			<button
				class="btn btn--secondary"
				type="button"
				disabled={(envelope.page ?? 1) <= 1}
				onclick={prevPage}
			>
				<CaretLeftIcon size={16} weight="bold" />
				<span>Prev</span>
			</button>
			<span class="pager__info">
				Page {envelope.page} / {envelope.total_pages || 1} · {summaryRange}
			</span>
			<button
				class="btn btn--secondary"
				type="button"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={nextPage}
			>
				<span>Next</span>
				<CaretRightIcon size={16} weight="bold" />
			</button>
		</div>
	{/if}

	{#if drawerOpen}
		<div
			class="drawer-backdrop"
			role="button"
			tabindex="-1"
			aria-label="Close"
			onclick={() => (drawerOpen = false)}
			onkeydown={(e) => e.key === 'Escape' && (drawerOpen = false)}
		></div>
		<aside class="drawer" aria-label="Add suppression">
			<header class="drawer__header">
				<h2 class="drawer__title">Add suppression</h2>
				<button
					class="btn btn--secondary btn--small"
					type="button"
					onclick={() => (drawerOpen = false)}
				>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
				</button>
			</header>
			<div class="form">
				<div class="field">
					<label class="field__label" for="supp-email">Email</label>
					<input
						id="supp-email"
						name="supp-email"
						type="email"
						class="field__input"
						placeholder="user@example.com"
						bind:value={formEmail}
						autocomplete="email"
						required
					/>
				</div>
				<div class="field">
					<label class="field__label" for="supp-reason">Reason</label>
					<input
						id="supp-reason"
						name="supp-reason"
						class="field__input"
						placeholder="bounce, complaint, ticket #1234, …"
						bind:value={formReason}
						required
					/>
				</div>
				<p class="hint">
					Sends to this address will fail with a permanent suppression error until the row
					is removed. The audit log captures who added it.
				</p>
				<div class="form__actions">
					<button
						class="btn btn--primary"
						type="button"
						disabled={formBusy}
						onclick={addSuppression}
					>
						<CheckCircleIcon size={16} weight="bold" />
						<span>{formBusy ? 'Adding…' : 'Suppress'}</span>
					</button>
					<button
						class="btn btn--secondary"
						type="button"
						onclick={() => (drawerOpen = false)}
					>
						<XIcon size={16} weight="bold" />
						<span>Cancel</span>
					</button>
				</div>
			</div>
		</aside>
	{/if}
</div>

<style>
	.page {
		max-width: 80rem;
		padding: 0 0 3rem;
	}
	.page__header {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		align-items: flex-start;
		justify-content: space-between;
		margin-bottom: 1.25rem;
	}
	.page__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
		flex: 1;
		min-width: 0;
	}
	.page__copy {
		min-width: 0;
	}
	.page__actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.eyebrow {
		display: inline-block;
		font-size: 0.6875rem;
		font-weight: 700;
		line-height: 1;
		letter-spacing: 0.08em;
		color: var(--color-grey-500);
		text-transform: uppercase;
		margin-bottom: 0.4rem;
	}
	.page__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.2;
	}
	.page__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
	}

	.error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		margin-bottom: 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}

	.state {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 4rem 0;
		color: var(--color-grey-400);
		font-size: 0.875rem;
	}
	.state__spinner {
		width: 1.25rem;
		height: 1.25rem;
		border: 2px solid rgba(255, 255, 255, 0.1);
		border-top-color: var(--color-teal);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 3rem 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-500);
		text-align: center;
	}
	.empty :global(svg) {
		color: var(--color-grey-500);
	}
	.empty__title {
		margin: 0.5rem 0 0;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
	}
	.empty__sub {
		margin: 0 0 0.75rem;
		font-size: 0.875rem;
		color: var(--color-grey-400);
	}

	.card {
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
		font-size: 0.875rem;
	}
	.table th {
		text-align: left;
		font-weight: 700;
		color: var(--color-grey-500);
		font-size: 0.6875rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.75rem 1rem;
		background: rgba(255, 255, 255, 0.02);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		white-space: nowrap;
	}
	.table td {
		padding: 0.875rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
		vertical-align: middle;
	}
	.table tbody tr:hover td {
		background: rgba(255, 255, 255, 0.02);
	}
	.table tbody tr:last-child td {
		border-bottom: none;
	}
	.table__actions-th {
		text-align: right;
	}
	.row-actions {
		display: flex;
		gap: 0.4rem;
		justify-content: flex-end;
		flex-wrap: wrap;
	}
	.ts {
		font-variant-numeric: tabular-nums;
		color: var(--color-grey-300);
		white-space: nowrap;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 3rem;
		padding: 0 1.25rem;
		border-radius: var(--radius-2xl);
		font-size: 0.8125rem;
		font-weight: 600;
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms,
			box-shadow 150ms,
			transform 150ms;
	}
	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
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
	.btn--danger {
		background: rgba(239, 68, 68, 0.1);
		color: #fca5a5;
		border-color: rgba(239, 68, 68, 0.3);
	}
	.btn--danger:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.18);
	}
	.btn--small {
		min-height: 2.5rem;
		padding: 0 0.65rem;
		font-size: 0.75rem;
	}

	.pager {
		display: flex;
		gap: 0.75rem;
		justify-content: center;
		align-items: center;
		margin-top: 1.25rem;
		flex-wrap: wrap;
	}
	.pager__info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
	}

	.drawer-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		z-index: 60;
	}
	.drawer {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		width: min(560px, 92vw);
		background: var(--color-navy);
		border-left: 1px solid rgba(255, 255, 255, 0.08);
		padding: 1.5rem;
		overflow-y: auto;
		z-index: 70;
		box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3);
	}
	.drawer__header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}
	.drawer__title {
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0;
		letter-spacing: -0.01em;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.form__actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		margin-top: 0.5rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.field__label {
		font-size: 0.75rem;
		color: var(--color-grey-300);
		font-weight: 500;
	}
	.field__input {
		min-height: 3rem;
		padding: 0 1.25rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		width: 100%;
		font-family: inherit;
		color-scheme: dark;
		transition:
			border-color 150ms,
			box-shadow 150ms;
	}
	.field__input::placeholder {
		color: var(--color-grey-500);
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}
	.hint {
		margin: 0;
		font-size: 0.75rem;
		color: var(--color-grey-500);
		line-height: 1.45;
	}
</style>
