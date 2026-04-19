<script lang="ts">
	import { onMount } from 'svelte';
	import Trash from 'phosphor-svelte/lib/Trash';
	import DownloadSimple from 'phosphor-svelte/lib/DownloadSimple';
	import ArrowClockwise from 'phosphor-svelte/lib/ArrowClockwise';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import XCircle from 'phosphor-svelte/lib/XCircle';
	import Warning from 'phosphor-svelte/lib/Warning';
	import { ApiError } from '$lib/api/client';
	import {
		dsarAdmin,
		type DsarJob,
		type DsarJobListEnvelope,
		type DsarListQuery
	} from '$lib/api/admin-security';

	let envelope = $state<DsarJobListEnvelope | null>(null);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');
	let selected = $state<DsarJob | null>(null);

	let filters = $state<DsarListQuery>({
		status: '',
		kind: '',
		target_user_id: '',
		limit: 25,
		offset: 0
	});

	let exportTarget = $state('');
	let exportReason = $state('');
	let exportBusy = $state(false);
	let exportPayload = $state<unknown>(null);

	let eraseTarget = $state('');
	let eraseReason = $state('');
	let eraseBusy = $state(false);

	let approveReason = $state('');
	let approveBusy = $state(false);
	let approvingId = $state<string | null>(null);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	function buildQuery(): DsarListQuery {
		const q: DsarListQuery = {
			limit: filters.limit ?? 25,
			offset: filters.offset ?? 0
		};
		if (filters.status?.trim()) q.status = filters.status.trim();
		if (filters.kind?.trim()) q.kind = filters.kind.trim();
		if (filters.target_user_id?.trim()) q.target_user_id = filters.target_user_id.trim();
		return q;
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await dsarAdmin.listJobs(buildQuery());
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load jobs';
		} finally {
			loading = false;
		}
	}

	function applyFilters(e: Event) {
		e.preventDefault();
		filters.offset = 0;
		void refresh();
	}

	async function createExport(e: Event) {
		e.preventDefault();
		if (!exportTarget.trim() || !exportReason.trim()) return;
		exportBusy = true;
		exportPayload = null;
		error = '';
		try {
			const res = await dsarAdmin.createExport({
				target_user_id: exportTarget.trim(),
				reason: exportReason.trim()
			});
			exportPayload = res.export;
			exportTarget = '';
			exportReason = '';
			flash('Export composed and persisted');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Export failed';
		} finally {
			exportBusy = false;
		}
	}

	async function requestErase(e: Event) {
		e.preventDefault();
		if (!eraseTarget.trim() || eraseReason.trim().length < 10) return;
		eraseBusy = true;
		error = '';
		try {
			await dsarAdmin.requestErase({
				target_user_id: eraseTarget.trim(),
				reason: eraseReason.trim()
			});
			eraseTarget = '';
			eraseReason = '';
			flash('Erasure queued — awaiting second-admin approval');
			await refresh();
		} catch (e) {
			error =
				e instanceof ApiError
					? `${e.status}: ${e.message}`
					: 'Failed to queue erasure';
		} finally {
			eraseBusy = false;
		}
	}

	async function approveErase(id: string) {
		if (approveReason.trim().length < 10) {
			error = 'Approval reason must be at least 10 characters';
			return;
		}
		approveBusy = true;
		error = '';
		try {
			await dsarAdmin.approveErase(id, { approval_reason: approveReason.trim() });
			flash('Erasure approved and tombstone applied');
			approvingId = null;
			approveReason = '';
			await refresh();
		} catch (e) {
			error =
				e instanceof ApiError
					? `${e.status}: ${e.message}`
					: 'Approval failed';
		} finally {
			approveBusy = false;
		}
	}

	async function cancelJob(job: DsarJob) {
		const reason = prompt('Optional cancel reason:') ?? undefined;
		try {
			await dsarAdmin.cancelJob(job.id, reason);
			flash('Job cancelled');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Cancel failed';
		}
	}

	function downloadArtifact(job: DsarJob) {
		if (!job.artifact_url) return;
		const a = document.createElement('a');
		a.href = job.artifact_url;
		a.download = `dsar-${job.target_user_id}-${job.id.slice(0, 8)}.json`;
		document.body.appendChild(a);
		a.click();
		a.remove();
	}

	function statusClass(status: string): string {
		switch (status) {
			case 'completed':
				return 'badge--ok';
			case 'pending':
				return 'badge--warn';
			case 'cancelled':
				return 'badge--off';
			case 'failed':
				return 'badge--err';
			default:
				return 'badge--off';
		}
	}

	function summaryRange(): string {
		if (!envelope) return '';
		const start = (filters.offset ?? 0) + 1;
		const end = (filters.offset ?? 0) + envelope.data.length;
		return `${start}–${end} of ${envelope.total}`;
	}

	function formatJson(v: unknown): string {
		try {
			return JSON.stringify(v, null, 2);
		} catch {
			return String(v);
		}
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>DSAR · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-dsar-page">
	<header class="page__header">
		<div class="page__title-row">
			<Trash size={28} weight="duotone" />
			<h1 class="page__title">DSAR &amp; right-to-erasure</h1>
		</div>
		<p class="page__subtitle">
			Operator-driven Data Subject Access Requests. Exports compose synchronously and persist
			a snapshot to the job row. Erasures are dual-control: one admin requests, a different
			admin approves and the tombstone runs in a transaction.
		</p>
	</header>

	{#if toast}
		<div class="toast">{toast}</div>
	{/if}
	{#if error}
		<div class="error" role="alert" data-testid="dsar-error">{error}</div>
	{/if}

	<div class="composers">
		<section class="card">
			<header class="card__heading">
				<DownloadSimple size={20} weight="duotone" />
				<h2 class="card__title">Compose export</h2>
			</header>
			<form class="form" onsubmit={createExport}>
				<div class="field">
					<label class="field__label" for="exp-target">Target user id (UUID)</label>
					<input
						id="exp-target"
						class="field__input"
						placeholder="00000000-…"
						bind:value={exportTarget}
						data-testid="dsar-export-target"
						required
					/>
				</div>
				<div class="field">
					<label class="field__label" for="exp-reason">Reason</label>
					<input
						id="exp-reason"
						class="field__input"
						placeholder="ticket #1234 — subject access request"
						bind:value={exportReason}
						data-testid="dsar-export-reason"
						required
					/>
				</div>
				<button
					class="btn btn--primary"
					type="submit"
					disabled={exportBusy}
					data-testid="dsar-export-submit"
				>
					{exportBusy ? 'Composing…' : 'Compose export'}
				</button>
			</form>
			{#if exportPayload}
				<details class="export-preview" open>
					<summary>Export snapshot</summary>
					<pre class="json">{formatJson(exportPayload)}</pre>
				</details>
			{/if}
		</section>

		<section class="card card--danger">
			<header class="card__heading">
				<Warning size={20} weight="duotone" />
				<h2 class="card__title">Request erasure</h2>
			</header>
			<p class="card__hint">
				Tombstones overwrite PII columns and drop session/credential rows. Requires a
				<strong>different</strong> admin to approve before the tombstone runs.
			</p>
			<form class="form" onsubmit={requestErase}>
				<div class="field">
					<label class="field__label" for="erase-target">Target user id (UUID)</label>
					<input
						id="erase-target"
						class="field__input"
						placeholder="00000000-…"
						bind:value={eraseTarget}
						data-testid="dsar-erase-target"
						required
					/>
				</div>
				<div class="field">
					<label class="field__label" for="erase-reason">
						Reason (≥ 10 characters)
					</label>
					<input
						id="erase-reason"
						class="field__input"
						placeholder="GDPR Art. 17 request — ticket #5678"
						bind:value={eraseReason}
						minlength="10"
						required
						data-testid="dsar-erase-reason"
					/>
				</div>
				<button
					class="btn btn--danger"
					type="submit"
					disabled={eraseBusy}
					data-testid="dsar-erase-submit"
				>
					{eraseBusy ? 'Queueing…' : 'Queue erasure'}
				</button>
			</form>
		</section>
	</div>

	<form class="filters" onsubmit={applyFilters}>
		<div class="filters__grid">
			<div class="field">
				<label class="field__label" for="dsar-status">Status</label>
				<select
					id="dsar-status"
					class="field__input"
					bind:value={filters.status}
				>
					<option value="">Any</option>
					<option value="pending">Pending</option>
					<option value="completed">Completed</option>
					<option value="cancelled">Cancelled</option>
					<option value="failed">Failed</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="dsar-kind">Kind</label>
				<select id="dsar-kind" class="field__input" bind:value={filters.kind}>
					<option value="">Any</option>
					<option value="export">Export</option>
					<option value="erase">Erase</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="dsar-target">Target user id</label>
				<input
					id="dsar-target"
					class="field__input"
					placeholder="UUID"
					bind:value={filters.target_user_id}
				/>
			</div>
			<div class="field field--actions">
				<button class="btn btn--ghost" type="submit">Apply filters</button>
				<button class="btn btn--ghost" type="button" onclick={refresh}>
					<ArrowClockwise size={16} weight="bold" />
					Refresh
				</button>
			</div>
		</div>
	</form>

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if !envelope || envelope.data.length === 0}
		<p class="muted">No DSAR jobs match the current filters.</p>
	{:else}
		<div class="card table-wrap">
			<table class="table" data-testid="dsar-table">
				<thead>
					<tr>
						<th>Created</th>
						<th>Kind</th>
						<th>Status</th>
						<th>Target</th>
						<th>Reason</th>
						<th aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each envelope.data as job (job.id)}
						<tr>
							<td>{new Date(job.created_at).toLocaleString()}</td>
							<td>
								<span class="kind kind--{job.kind}">{job.kind}</span>
							</td>
							<td>
								<span class="badge {statusClass(job.status)}">{job.status}</span>
							</td>
							<td>
								<code title={job.target_user_id}>
									{job.target_user_id.slice(0, 8)}…
								</code>
							</td>
							<td class="reason-cell" title={job.request_reason}>
								{job.request_reason}
							</td>
							<td class="row-actions">
								<button
									class="btn btn--ghost btn--small"
									onclick={() => (selected = job)}
								>
									Inspect
								</button>
								{#if job.kind === 'export' && job.artifact_url}
									<button
										class="btn btn--ghost btn--small"
										onclick={() => downloadArtifact(job)}
										title="Download composed export"
									>
										<DownloadSimple size={14} weight="bold" />
									</button>
								{/if}
								{#if job.kind === 'erase' && job.status === 'pending'}
									<button
										class="btn btn--primary btn--small"
										onclick={() => {
											approvingId = job.id;
											approveReason = '';
										}}
										data-testid="dsar-approve"
									>
										<CheckCircle size={14} weight="bold" />
										Approve
									</button>
									<button
										class="btn btn--ghost btn--small"
										onclick={() => cancelJob(job)}
									>
										<XCircle size={14} weight="bold" />
										Cancel
									</button>
								{/if}
							</td>
						</tr>

						{#if approvingId === job.id}
							<tr class="approve-row">
								<td colspan="6">
									<div class="approve-form">
										<input
											class="field__input"
											placeholder="Approval reason (≥ 10 chars)"
											bind:value={approveReason}
											minlength="10"
										/>
										<button
											class="btn btn--danger"
											onclick={() => approveErase(job.id)}
											disabled={approveBusy}
											data-testid="dsar-approve-confirm"
										>
											{approveBusy ? 'Tombstoning…' : 'Confirm tombstone'}
										</button>
										<button
											class="btn btn--ghost"
											onclick={() => {
												approvingId = null;
												approveReason = '';
											}}
										>
											Cancel
										</button>
									</div>
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>

		<div class="pager">
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) <= 1}
				onclick={() => {
					filters.offset = Math.max(0, (filters.offset ?? 0) - (filters.limit ?? 25));
					void refresh();
				}}
			>
				Prev
			</button>
			<span class="pager__info">
				Page {envelope.page} / {envelope.total_pages || 1} · {summaryRange()}
			</span>
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={() => {
					filters.offset = (filters.offset ?? 0) + (filters.limit ?? 25);
					void refresh();
				}}
			>
				Next
			</button>
		</div>
	{/if}

	{#if selected}
		<div
			class="drawer-backdrop"
			role="button"
			tabindex="-1"
			aria-label="Close inspector"
			onclick={() => (selected = null)}
			onkeydown={(e) => e.key === 'Escape' && (selected = null)}
		></div>
		<aside class="drawer" data-testid="dsar-drawer" aria-label="DSAR job detail">
			<header class="drawer__header">
				<h2 class="drawer__title">DSAR job</h2>
				<button class="btn btn--ghost btn--small" onclick={() => (selected = null)}>
					Close
				</button>
			</header>
			<dl class="drawer__meta">
				<dt>Id</dt><dd><code>{selected.id}</code></dd>
				<dt>Kind</dt><dd>{selected.kind}</dd>
				<dt>Status</dt><dd>{selected.status}</dd>
				<dt>Target</dt><dd><code>{selected.target_user_id}</code></dd>
				<dt>Created</dt><dd>{new Date(selected.created_at).toLocaleString()}</dd>
				<dt>Updated</dt><dd>{new Date(selected.updated_at).toLocaleString()}</dd>
				<dt>Requested by</dt><dd><code>{selected.requested_by}</code></dd>
				<dt>Request reason</dt><dd>{selected.request_reason}</dd>
				{#if selected.approved_by}
					<dt>Approved by</dt><dd><code>{selected.approved_by}</code></dd>
				{/if}
				{#if selected.approval_reason}
					<dt>Approval reason</dt><dd>{selected.approval_reason}</dd>
				{/if}
				{#if selected.completed_at}
					<dt>Completed</dt><dd>{new Date(selected.completed_at).toLocaleString()}</dd>
				{/if}
				{#if selected.failure_reason}
					<dt>Failure</dt><dd>{selected.failure_reason}</dd>
				{/if}
			</dl>
			{#if selected.erasure_summary}
				<details open>
					<summary>Erasure summary</summary>
					<pre class="json">{formatJson(selected.erasure_summary)}</pre>
				</details>
			{/if}
		</aside>
	{/if}
</div>

<style>
	.page {
		max-width: 1280px;
	}
	.page__header {
		margin-bottom: var(--space-5);
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
		max-width: 75ch;
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
	.composers {
		display: grid;
		grid-template-columns: 1fr;
		gap: var(--space-4);
		margin-bottom: var(--space-5);
	}
	@media (min-width: 900px) {
		.composers {
			grid-template-columns: 1fr 1fr;
		}
	}
	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-5);
	}
	.card--danger {
		border-color: rgba(239, 68, 68, 0.25);
		background: linear-gradient(180deg, rgba(239, 68, 68, 0.04), var(--color-navy-mid));
	}
	.card__heading {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-white);
		margin-bottom: var(--space-3);
	}
	.card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
	}
	.card__hint {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-bottom: var(--space-3);
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field--actions {
		flex-direction: row;
		align-items: end;
		gap: var(--space-2);
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
		width: 100%;
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
		opacity: 0.4;
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
	.btn--ghost:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}
	.btn--danger {
		background: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
		border-color: rgba(239, 68, 68, 0.25);
	}
	.btn--danger:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.18);
	}
	.btn--small {
		padding: 0.25rem 0.6rem;
		font-size: var(--fs-xs);
	}
	.export-preview {
		margin-top: var(--space-3);
	}
	.json {
		background: rgba(0, 0, 0, 0.3);
		padding: var(--space-3);
		border-radius: var(--radius-lg);
		font-size: 0.72rem;
		color: var(--color-grey-200);
		max-height: 40vh;
		overflow: auto;
		white-space: pre-wrap;
		word-break: break-all;
	}
	.filters {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-4);
		margin-bottom: var(--space-4);
	}
	.filters__grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: var(--space-3);
		align-items: end;
	}
	.table-wrap {
		overflow-x: auto;
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
		white-space: nowrap;
	}
	.table td {
		padding: var(--space-3) var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
		vertical-align: middle;
	}
	.kind {
		display: inline-block;
		padding: 0.1rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.kind--export {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}
	.kind--erase {
		background: rgba(239, 68, 68, 0.15);
		color: #fca5a5;
	}
	.badge {
		display: inline-block;
		padding: 0.1rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.badge--ok {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}
	.badge--warn {
		background: rgba(245, 158, 11, 0.15);
		color: #fbbf24;
	}
	.badge--off {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
	.badge--err {
		background: rgba(239, 68, 68, 0.15);
		color: #fca5a5;
	}
	.reason-cell {
		max-width: 28ch;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.row-actions {
		display: flex;
		gap: var(--space-2);
		justify-content: flex-end;
	}
	.approve-row td {
		background: rgba(245, 158, 11, 0.05);
	}
	.approve-form {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.approve-form .field__input {
		flex: 1;
	}
	.pager {
		display: flex;
		gap: var(--space-3);
		justify-content: center;
		align-items: center;
		margin-top: var(--space-4);
	}
	.pager__info {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
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
		padding: var(--space-5);
		overflow-y: auto;
		z-index: 70;
		box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3);
	}
	.drawer__header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}
	.drawer__title {
		font-size: var(--fs-lg);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
	}
	.drawer__meta {
		display: grid;
		grid-template-columns: 8rem 1fr;
		gap: var(--space-2) var(--space-3);
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
		margin-bottom: var(--space-4);
	}
	.drawer__meta dt {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.drawer__meta dd {
		margin: 0;
		word-break: break-all;
	}
</style>
