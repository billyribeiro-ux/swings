<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import DownloadSimpleIcon from 'phosphor-svelte/lib/DownloadSimpleIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
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
	let exportAsync = $state(false);
	let pollingTimer: ReturnType<typeof setInterval> | null = null;
	let downloadingId = $state<string | null>(null);

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
				reason: exportReason.trim(),
				async: exportAsync
			});
			exportPayload = res.export;
			exportTarget = '';
			exportReason = '';
			flash(
				exportAsync
					? `Export queued — job ${res.job.id.slice(0, 8)}…; the worker will compose it shortly`
					: 'Export composed and persisted'
			);
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
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to queue erasure';
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
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Approval failed';
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

	async function downloadArtifact(job: DsarJob) {
		const kind = job.artifact_kind ?? 'inline';
		const fallbackName = `dsar-${job.target_user_id}-${job.id.slice(0, 8)}.json`;
		try {
			if (kind === 'local') {
				downloadingId = job.id;
				const { blob, filename } = await dsarAdmin.streamArtifact(job.id);
				const url = URL.createObjectURL(blob);
				try {
					const a = document.createElement('a');
					a.href = url;
					a.download = filename ?? fallbackName;
					document.body.appendChild(a);
					a.click();
					a.remove();
				} finally {
					URL.revokeObjectURL(url);
				}
				return;
			}
			if (!job.artifact_url) return;
			const a = document.createElement('a');
			a.href = job.artifact_url;
			a.download = fallbackName;
			document.body.appendChild(a);
			a.click();
			a.remove();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Download failed';
		} finally {
			downloadingId = null;
		}
	}

	function expiryHint(job: DsarJob): string | null {
		if (!job.artifact_expires_at) return null;
		const ms = new Date(job.artifact_expires_at).getTime() - Date.now();
		if (Number.isNaN(ms)) return null;
		if (ms <= 0) return 'expired';
		const hours = Math.floor(ms / 3_600_000);
		if (hours >= 24) return `expires in ${Math.floor(hours / 24)}d`;
		if (hours >= 1) return `expires in ${hours}h`;
		const mins = Math.max(1, Math.floor(ms / 60_000));
		return `expires in ${mins}m`;
	}

	function isArtifactExpired(job: DsarJob): boolean {
		if (!job.artifact_expires_at) return false;
		return new Date(job.artifact_expires_at).getTime() <= Date.now();
	}

	function statusClass(status: string): string {
		switch (status) {
			case 'completed':
				return 'pill--success';
			case 'pending':
			case 'composing':
				return 'pill--warn';
			case 'cancelled':
				return 'pill--neutral';
			case 'failed':
				return 'pill--danger';
			default:
				return 'pill--neutral';
		}
	}

	// Polling for in-flight DSAR jobs.
	//
	// The previous `$effect` here read `envelope` (reactive) and wrote
	// `pollingTimer` (a plain `let`, NOT $state). On every `refresh()` the
	// interval fired, `envelope` changed, the effect's teardown ran, the body
	// ran, a NEW interval was scheduled. Worse, the teardown clobbered
	// `pollingTimer` *after* the body had assigned it — and because
	// `pollingTimer` is not reactive, the read inside the body and the write
	// inside the teardown could race when multiple `envelope` updates landed
	// in the same microtask, leaving an orphan interval that fired forever
	// and called `refresh()` (which sets `loading=true`) — that's why the
	// dashboard skeletons could appear stuck.
	//
	// New shape: read `envelope` only; let `untrack` quarantine the imperative
	// timer bookkeeping so this effect cannot retrigger via its own writes.
	$effect(() => {
		const inFlight =
			envelope?.data.some((j) => j.status === 'pending' || j.status === 'composing') ?? false;
		untrack(() => {
			if (inFlight) {
				if (pollingTimer === null) {
					pollingTimer = setInterval(() => {
						void refresh();
					}, 5000);
				}
			} else if (pollingTimer !== null) {
				clearInterval(pollingTimer);
				pollingTimer = null;
			}
		});
		return () => {
			if (pollingTimer !== null) {
				clearInterval(pollingTimer);
				pollingTimer = null;
			}
		};
	});

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
			<TrashIcon size={28} weight="duotone" />
			<div class="page__copy">
				<h1 class="page__title">DSAR &amp; right-to-erasure</h1>
				<p class="page__subtitle">
					Operator-driven Data Subject Access Requests. Exports compose synchronously or via a
					background worker. Erasures are dual-control: one admin requests, a different admin
					approves.
				</p>
			</div>
		</div>
	</header>

	{#if toast}
		<div class="toast" role="status">
			<CheckCircleIcon size={16} weight="fill" />
			<span>{toast}</span>
		</div>
	{/if}
	{#if error}
		<div class="error" role="alert" data-testid="dsar-error">
			<WarningIcon size={16} weight="fill" />
			<span>{error}</span>
		</div>
	{/if}

	<div class="composers">
		<section class="card">
			<header class="card__head">
				<span class="card__eyebrow">New request</span>
				<h2 class="card__title">
					<DownloadSimpleIcon size={18} weight="duotone" />
					Compose export
				</h2>
				<p class="card__hint">Snapshot a member's data for delivery or audit response.</p>
			</header>
			<form class="form" onsubmit={createExport}>
				<div class="field">
					<label class="field__label" for="exp-target">Target user id (UUID)</label>
					<input
						id="exp-target"
						name="exp-target"
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
						name="exp-reason"
						class="field__input"
						placeholder="ticket #1234 — subject access request"
						bind:value={exportReason}
						data-testid="dsar-export-reason"
						required
					/>
				</div>
				<label class="async-toggle" for="exp-async">
					<input
						id="exp-async"
						name="exp-async"
						type="checkbox"
						bind:checked={exportAsync}
						data-testid="dsar-export-async"
					/>
					<span>
						Compose async (queue a worker job — recommended for large tenants)
					</span>
				</label>
				<button
					class="btn btn--primary"
					type="submit"
					disabled={exportBusy}
					data-testid="dsar-export-submit"
				>
					<PlusIcon size={16} weight="bold" />
					<span>
						{exportBusy
							? exportAsync
								? 'Queueing…'
								: 'Composing…'
							: exportAsync
								? 'Queue export'
								: 'Compose export'}
					</span>
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
			<header class="card__head">
				<span class="card__eyebrow card__eyebrow--danger">Destructive</span>
				<h2 class="card__title">
					<WarningIcon size={18} weight="duotone" />
					Request erasure
				</h2>
				<p class="card__hint">
					Tombstones overwrite PII columns and drop session/credential rows. Requires a
					<strong>different</strong> admin to approve before the tombstone runs.
				</p>
			</header>
			<form class="form" onsubmit={requestErase}>
				<div class="field">
					<label class="field__label" for="erase-target">Target user id (UUID)</label>
					<input
						id="erase-target"
						name="erase-target"
						class="field__input"
						placeholder="00000000-…"
						bind:value={eraseTarget}
						data-testid="dsar-erase-target"
						required
					/>
				</div>
				<div class="field">
					<label class="field__label" for="erase-reason">Reason (≥ 10 characters)</label>
					<input
						id="erase-reason"
						name="erase-reason"
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
					<TrashIcon size={16} weight="bold" />
					<span>{eraseBusy ? 'Queueing…' : 'Queue erasure'}</span>
				</button>
			</form>
		</section>
	</div>

	<form class="filters" onsubmit={applyFilters}>
		<header class="filters__head">
			<span class="filters__eyebrow">
				<FunnelIcon size={14} weight="bold" />
				Filters
			</span>
		</header>
		<div class="filters__grid">
			<div class="field">
				<label class="field__label" for="dsar-status">Status</label>
				<select
					id="dsar-status"
					name="dsar-status"
					class="field__input"
					bind:value={filters.status}
				>
					<option value="">Any</option>
					<option value="pending">Pending</option>
					<option value="composing">Composing</option>
					<option value="completed">Completed</option>
					<option value="cancelled">Cancelled</option>
					<option value="failed">Failed</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="dsar-kind">Kind</label>
				<select
					id="dsar-kind"
					name="dsar-kind"
					class="field__input"
					bind:value={filters.kind}
				>
					<option value="">Any</option>
					<option value="export">Export</option>
					<option value="erase">Erase</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="dsar-target">Target user id</label>
				<input
					id="dsar-target"
					name="dsar-target"
					class="field__input"
					placeholder="UUID"
					bind:value={filters.target_user_id}
				/>
			</div>
		</div>
		<div class="filters__actions">
			<button class="btn btn--primary" type="submit">
				<FunnelIcon size={16} weight="bold" />
				<span>Apply</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={refresh}>
				<ArrowsClockwiseIcon size={16} weight="bold" />
				<span>Refresh</span>
			</button>
		</div>
	</form>

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading DSAR jobs…</span>
		</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty">
			<TrashIcon size={48} weight="duotone" />
			<p class="empty__title">No DSAR jobs</p>
			<p class="empty__sub">Compose an export or queue an erasure to get started.</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table" data-testid="dsar-table">
					<thead>
						<tr>
							<th scope="col">Created</th>
							<th scope="col">Kind</th>
							<th scope="col">Status</th>
							<th scope="col">Target</th>
							<th scope="col">Reason</th>
							<th scope="col" class="table__actions-th" aria-label="Actions"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.data as job (job.id)}
							<tr>
								<td class="table__ts">{new Date(job.created_at).toLocaleString()}</td>
								<td>
									<span class="pill pill--kind pill--kind-{job.kind}">{job.kind}</span>
								</td>
								<td>
									<span class="pill {statusClass(job.status)}">{job.status}</span>
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
										class="btn btn--secondary btn--small"
										onclick={() => (selected = job)}
										aria-label="Inspect job"
									>
										<EyeIcon size={14} weight="bold" />
										<span>Inspect</span>
									</button>
									{#if job.kind === 'export' && (job.artifact_url || job.artifact_kind === 'local')}
										{@const expired = isArtifactExpired(job)}
										{@const hint = expiryHint(job)}
										<button
											class="btn btn--secondary btn--small"
											onclick={() => downloadArtifact(job)}
											disabled={expired || downloadingId === job.id}
											title={expired
												? 'Artefact expired and was swept by ADM-19'
												: `Download composed export${hint ? ` (${hint})` : ''}`}
											data-testid="dsar-download"
											aria-label="Download artefact"
										>
											<DownloadSimpleIcon size={14} weight="bold" />
											<span>
												{#if downloadingId === job.id}
													…
												{:else}
													Download{#if hint}<span class="ttl-hint"> · {hint}</span>{/if}
												{/if}
											</span>
										</button>
									{/if}
									{#if job.kind === 'export' && (job.status === 'pending' || job.status === 'composing')}
										<span class="pill pill--warn" title="Worker in flight">queued</span>
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
											<CheckCircleIcon size={14} weight="bold" />
											<span>Approve</span>
										</button>
										<button
											class="btn btn--secondary btn--small"
											onclick={() => cancelJob(job)}
										>
											<XCircleIcon size={14} weight="bold" />
											<span>Cancel</span>
										</button>
									{/if}
								</td>
							</tr>

							{#if approvingId === job.id}
								<tr class="approve-row">
									<td colspan="6">
										<div class="approve-form">
											<input
												id="approve-reason-{job.id}"
												name="approve-reason-{job.id}"
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
												<TrashIcon size={14} weight="bold" />
												<span>{approveBusy ? 'Tombstoning…' : 'Confirm tombstone'}</span>
											</button>
											<button
												class="btn btn--secondary"
												onclick={() => {
													approvingId = null;
													approveReason = '';
												}}
											>
												<XIcon size={14} weight="bold" />
												<span>Cancel</span>
											</button>
										</div>
									</td>
								</tr>
							{/if}
						{/each}
					</tbody>
				</table>
			</div>
		</section>

		<div class="pager">
			<button
				class="btn btn--secondary"
				disabled={(envelope.page ?? 1) <= 1}
				onclick={() => {
					filters.offset = Math.max(0, (filters.offset ?? 0) - (filters.limit ?? 25));
					void refresh();
				}}
			>
				<span>Prev</span>
			</button>
			<span class="pager__info">
				Page {envelope.page} / {envelope.total_pages || 1} · {summaryRange()}
			</span>
			<button
				class="btn btn--secondary"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={() => {
					filters.offset = (filters.offset ?? 0) + (filters.limit ?? 25);
					void refresh();
				}}
			>
				<span>Next</span>
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
				<button class="btn btn--secondary btn--small" onclick={() => (selected = null)}>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
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
				{#if selected.artifact_kind}
					<dt>Artefact</dt>
					<dd>
						<code>{selected.artifact_kind}</code>
						{#if selected.artifact_storage_key}
							·&nbsp;<code title={selected.artifact_storage_key}>
								{selected.artifact_storage_key.length > 32
									? selected.artifact_storage_key.slice(0, 32) + '…'
									: selected.artifact_storage_key}
							</code>
						{/if}
					</dd>
				{/if}
				{#if selected.artifact_expires_at}
					<dt>Expires</dt>
					<dd>
						{new Date(selected.artifact_expires_at).toLocaleString()}
						{#if expiryHint(selected)}
							·&nbsp;<span class="muted-inline">{expiryHint(selected)}</span>
						{/if}
					</dd>
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
		max-width: 80rem;
		padding: 0 0 3rem;
	}
	.page__header {
		margin-bottom: 1.25rem;
	}
	.page__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.page__copy {
		min-width: 0;
	}
	.page__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.15;
	}
	.page__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 60ch;
		line-height: 1.55;
	}

	.toast,
	.error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		margin-bottom: 1rem;
	}
	.toast {
		background: rgba(15, 164, 175, 0.12);
		border: 1px solid rgba(15, 164, 175, 0.25);
		color: #5eead4;
	}
	.error {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}

	.composers {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.card--danger {
		border-color: rgba(239, 68, 68, 0.25);
		background: linear-gradient(180deg, rgba(239, 68, 68, 0.05), var(--color-navy-mid));
	}
	.card__head {
		margin-bottom: 1rem;
	}
	.card__eyebrow {
		display: inline-block;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin-bottom: 0.4rem;
	}
	.card__eyebrow--danger {
		color: #fca5a5;
	}
	.card__title {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
	}
	.card__hint {
		margin: 0.4rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		line-height: 1.55;
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
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
		min-height: 2.5rem;
		padding: 0.65rem 0.875rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-lg);
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
	.async-toggle {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
		font-size: 0.75rem;
		color: var(--color-grey-300);
		cursor: pointer;
		line-height: 1.45;
	}
	.async-toggle input {
		margin-top: 2px;
		accent-color: var(--color-teal);
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0 0.875rem;
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
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
		min-height: 2rem;
		padding: 0 0.65rem;
		font-size: 0.75rem;
	}

	.export-preview {
		margin-top: 1rem;
	}

	.json {
		background: rgba(0, 0, 0, 0.3);
		padding: 0.85rem;
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
		padding: 1.25rem;
		margin-bottom: 1.25rem;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.filters__head {
		margin-bottom: 0.85rem;
	}
	.filters__eyebrow {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.filters__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.85rem;
	}
	.filters__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 1rem;
		align-items: center;
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
		background: var(--color-navy-mid);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		color: var(--color-grey-500);
		text-align: center;
	}
	.empty__title {
		margin: 0.5rem 0 0;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
	}
	.empty__sub {
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
	}

	.table-card {
		overflow: hidden;
		padding: 0;
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
	.table__actions-th {
		text-align: right;
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
	.table__ts {
		font-variant-numeric: tabular-nums;
		color: var(--color-grey-300);
		white-space: nowrap;
	}

	.pill {
		display: inline-flex;
		align-items: center;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.pill--success {
		background: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}
	.pill--warn {
		background: rgba(245, 158, 11, 0.12);
		color: #fcd34d;
	}
	.pill--danger {
		background: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
	}
	.pill--neutral {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}
	.pill--kind-export {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}
	.pill--kind-erase {
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
		gap: 0.4rem;
		justify-content: flex-end;
		flex-wrap: wrap;
	}
	.approve-row td {
		background: rgba(245, 158, 11, 0.06);
	}
	.approve-form {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}
	.approve-form .field__input {
		flex: 1;
		min-width: 16rem;
	}

	.ttl-hint {
		font-size: 0.75em;
		color: var(--color-grey-400);
		margin-left: 4px;
	}
	.muted-inline {
		color: var(--color-grey-400);
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
		font-weight: 700;
		color: var(--color-white);
		margin: 0;
		letter-spacing: -0.01em;
	}
	.drawer__meta {
		display: grid;
		grid-template-columns: 8rem 1fr;
		gap: 0.5rem 0.85rem;
		font-size: 0.875rem;
		color: var(--color-grey-200);
		margin-bottom: 1rem;
	}
	.drawer__meta dt {
		color: var(--color-grey-500);
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.drawer__meta dd {
		margin: 0;
		word-break: break-all;
	}

	@media (min-width: 480px) {
		.filters__grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (min-width: 768px) {
		.card,
		.filters {
			padding: 1.75rem;
			border-radius: var(--radius-2xl);
		}
		.filters__grid {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}
	}
	@media (min-width: 900px) {
		.composers {
			grid-template-columns: 1fr 1fr;
		}
	}
</style>
