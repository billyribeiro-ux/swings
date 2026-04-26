<!--
  Phase 2.4 — Form submissions admin. Wraps
    GET  /api/admin/forms/{id}/submissions             (list, filter, CSV)
    POST /api/admin/forms/{id}/submissions/bulk        (mark_spam / restore / delete)
  Per-row inspector drawer + bulk-action bar + CSV export.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import ClipboardTextIcon from 'phosphor-svelte/lib/ClipboardTextIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import DownloadSimpleIcon from 'phosphor-svelte/lib/DownloadSimpleIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import ProhibitIcon from 'phosphor-svelte/lib/ProhibitIcon';
	import ArrowUUpLeftIcon from 'phosphor-svelte/lib/ArrowUUpLeftIcon';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import { getPublicApiBase } from '$lib/api/publicApiBase';
	import { ApiError } from '$lib/api/client';
	import { SvelteSet } from 'svelte/reactivity';
	import {
		formSubmissions,
		type BulkAction,
		type PaginatedSubmissions,
		type SubmissionListQuery,
		type SubmissionRow
	} from '$lib/api/admin-form-submissions';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	const formId = $derived(page.params.id ?? '');

	let envelope = $state<PaginatedSubmissions | null>(null);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let filters = $state<SubmissionListQuery>({
		status: '',
		from: '',
		to: '',
		page: 1,
		per_page: 25
	});

	let selected = $state<SubmissionRow | null>(null);
	const selectedIds: SvelteSet<string> = new SvelteSet();
	let bulkAction = $state<BulkAction>('mark_spam');
	let bulkBusy = $state(false);
	let exportBusy = $state(false);
	let rowBusy = $state<Record<string, BulkAction | null>>({});

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	function buildQuery(): SubmissionListQuery {
		const q: SubmissionListQuery = {
			page: filters.page ?? 1,
			per_page: filters.per_page ?? 25
		};
		if (filters.status?.trim()) q.status = filters.status.trim();
		if (filters.from) q.from = new Date(filters.from).toISOString();
		if (filters.to) q.to = new Date(filters.to).toISOString();
		return q;
	}

	async function refresh() {
		if (!formId) return;
		loading = true;
		error = '';
		try {
			envelope = await formSubmissions.list(formId, buildQuery());
			selectedIds.clear();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load submissions';
		} finally {
			loading = false;
		}
	}

	function applyFilters(e: Event) {
		e.preventDefault();
		filters.page = 1;
		void refresh();
	}

	function clearFilters() {
		filters = { status: '', from: '', to: '', page: 1, per_page: 25 };
		void refresh();
	}

	function toggleRow(id: string) {
		if (selectedIds.has(id)) selectedIds.delete(id);
		else selectedIds.add(id);
	}

	function toggleAll() {
		if (!envelope) return;
		if (selectedIds.size === envelope.data.length) {
			selectedIds.clear();
		} else {
			selectedIds.clear();
			for (const r of envelope.data) selectedIds.add(r.id);
		}
	}

	async function runBulk() {
		if (!formId) return;
		if (selectedIds.size === 0) return;
		if (bulkAction === 'delete') {
			const ok = await confirmDialog({
				title: `Permanently delete ${selectedIds.size} submission${selectedIds.size === 1 ? '' : 's'}?`,
				message: 'The submissions and any attached payload will be removed. This cannot be undone.',
				confirmLabel: 'Delete permanently',
				variant: 'danger'
			});
			if (!ok) return;
		}
		bulkBusy = true;
		error = '';
		try {
			const r = await formSubmissions.bulkAction(formId, {
				ids: Array.from(selectedIds),
				action: bulkAction
			});
			flash(`Updated ${r.updated} submission${r.updated === 1 ? '' : 's'}`);
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Bulk action failed';
		} finally {
			bulkBusy = false;
		}
	}

	async function rowAction(row: SubmissionRow, action: BulkAction) {
		if (!formId) return;
		if (action === 'delete') {
			const ok = await confirmDialog({
				title: `Permanently delete submission ${row.id.slice(0, 8)}…?`,
				message: 'The submission and any attached payload will be removed. This cannot be undone.',
				confirmLabel: 'Delete permanently',
				variant: 'danger'
			});
			if (!ok) return;
		}
		rowBusy = { ...rowBusy, [row.id]: action };
		error = '';
		try {
			await formSubmissions.bulkAction(formId, { ids: [row.id], action });
			flash('Updated');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Action failed';
		} finally {
			rowBusy = { ...rowBusy, [row.id]: null };
		}
	}

	async function exportCsv() {
		if (!formId) return;
		exportBusy = true;
		error = '';
		try {
			const url = `${getPublicApiBase()}${formSubmissions.csvExportUrl(formId, buildQuery())}`;
			// BFF (Phase 1.3): cookie-based auth — `credentials: 'include'`
			// ships the httpOnly `swings_access` cookie automatically.
			const res = await fetch(url, { credentials: 'include' });
			if (!res.ok) {
				error = `Export failed (${res.status})`;
				return;
			}
			const blob = await res.blob();
			const a = document.createElement('a');
			const objUrl = URL.createObjectURL(blob);
			a.href = objUrl;
			a.download = `form_${formId}_submissions_${new Date().toISOString().slice(0, 10)}.csv`;
			document.body.appendChild(a);
			a.click();
			a.remove();
			URL.revokeObjectURL(objUrl);
		} catch {
			error = 'CSV export failed';
		} finally {
			exportBusy = false;
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

	function statusPill(status: string): string {
		switch (status) {
			case 'complete':
				return 'pill pill--success';
			case 'spam':
				return 'pill pill--danger';
			case 'deleted':
				return 'pill pill--neutral';
			case 'partial':
				return 'pill pill--warn';
			default:
				return 'pill pill--neutral';
		}
	}

	function formatJson(v: unknown): string {
		try {
			return JSON.stringify(v, null, 2);
		} catch {
			return String(v);
		}
	}

	function submitterLabel(row: SubmissionRow): string {
		if (row.subject_id) return row.subject_id.slice(0, 8) + '…';
		if (row.anonymous_id) return 'anon ' + row.anonymous_id.slice(0, 6) + '…';
		return '—';
	}

	const summaryRange = $derived.by(() => {
		if (!envelope) return '';
		const start = ((envelope.page ?? 1) - 1) * (envelope.per_page ?? 25) + 1;
		const end = start + envelope.data.length - 1;
		return `${start}–${end} of ${envelope.total}`;
	});

	const allSelected = $derived(
		envelope ? envelope.data.length > 0 && selectedIds.size === envelope.data.length : false
	);

	onMount(() => {
		void refresh();
	});
</script>

<svelte:head>
	<title>Submissions · Form · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-form-submissions">
	<header class="page__header">
		<div class="page__title-row">
			<ClipboardTextIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Operations / Forms</span>
				<h1 class="page__title">Submissions</h1>
				<p class="page__subtitle">
					Every submission against this form. Inspect the captured data, mark spam,
					restore, or hard-delete. CSV export honours the active filter set.
				</p>
				<a class="back-link" href="/admin/forms/{formId}">
					<ArrowLeftIcon size={14} weight="bold" />
					<span>Back to form</span>
				</a>
			</div>
		</div>
		<div class="page__actions">
			<button class="btn btn--secondary" type="button" onclick={exportCsv} disabled={exportBusy}>
				<DownloadSimpleIcon size={16} weight="bold" />
				<span>{exportBusy ? 'Exporting…' : 'Export CSV'}</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={() => void refresh()}>
				<ArrowsClockwiseIcon size={16} weight="bold" />
				<span>Refresh</span>
			</button>
		</div>
	</header>

	{#if toast}
		<div class="toast" role="status">
			<CheckCircleIcon size={16} weight="fill" />
			<span>{toast}</span>
		</div>
	{/if}
	{#if error}
		<div class="error" role="alert">
			<WarningIcon size={16} weight="fill" />
			<span>{error}</span>
		</div>
	{/if}

	<form class="filters" onsubmit={applyFilters}>
		<header class="filters__head">
			<span class="filters__eyebrow">
				<FunnelIcon size={14} weight="bold" />
				Filters
			</span>
		</header>
		<div class="filters__grid">
			<div class="field">
				<label class="field__label" for="sub-status">Status</label>
				<select
					id="sub-status"
					name="sub-status"
					class="field__input"
					bind:value={filters.status}
				>
					<option value="">All</option>
					<option value="complete">Complete</option>
					<option value="partial">Partial</option>
					<option value="spam">Spam</option>
					<option value="deleted">Deleted</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="sub-from">From</label>
				<input
					id="sub-from"
					name="sub-from"
					type="datetime-local"
					class="field__input"
					bind:value={filters.from}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="sub-to">To</label>
				<input
					id="sub-to"
					name="sub-to"
					type="datetime-local"
					class="field__input"
					bind:value={filters.to}
				/>
			</div>
		</div>
		<div class="filters__actions">
			<button class="btn btn--primary" type="submit">
				<MagnifyingGlassIcon size={16} weight="bold" />
				<span>Apply</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={clearFilters}>
				<XIcon size={16} weight="bold" />
				<span>Clear</span>
			</button>
		</div>
	</form>

	{#if selectedIds.size > 0}
		<div class="bulkbar" role="region" aria-label="Bulk actions">
			<span class="bulkbar__count">{selectedIds.size} selected</span>
			<div class="bulkbar__group">
				<label class="field__label" for="bulk-action">Action</label>
				<select
					id="bulk-action"
					name="bulk-action"
					class="field__input field__input--inline"
					bind:value={bulkAction}
				>
					<option value="mark_spam">Mark as spam</option>
					<option value="restore">Restore</option>
					<option value="delete">Delete</option>
				</select>
			</div>
			<button class="btn btn--primary" type="button" onclick={runBulk} disabled={bulkBusy}>
				<CheckCircleIcon size={16} weight="bold" />
				<span>{bulkBusy ? 'Applying…' : 'Apply'}</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={() => selectedIds.clear()}>
				<XIcon size={16} weight="bold" />
				<span>Clear selection</span>
			</button>
		</div>
	{/if}

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading submissions…</span>
		</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty">
			<ClipboardTextIcon size={48} weight="duotone" />
			<p class="empty__title">No submissions match</p>
			<p class="empty__sub">Adjust filters or wait for traffic to land.</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col" class="table__select-th">
								<input
									type="checkbox"
									checked={allSelected}
									onchange={toggleAll}
									aria-label="Select all"
								/>
							</th>
							<th scope="col">Submitted</th>
							<th scope="col">Submitter</th>
							<th scope="col">IP hash</th>
							<th scope="col">Status</th>
							<th scope="col" class="table__actions-th" aria-label="Actions"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.data as row (row.id)}
							<tr>
								<td>
									<input
										type="checkbox"
										checked={selectedIds.has(row.id)}
										onchange={() => toggleRow(row.id)}
										aria-label="Select row"
									/>
								</td>
								<td class="ts" title={row.submitted_at}>
									{new Date(row.submitted_at).toLocaleString()}
								</td>
								<td><code>{submitterLabel(row)}</code></td>
								<td><code title={row.ip_hash}>{row.ip_hash.slice(0, 12)}…</code></td>
								<td><span class={statusPill(row.status)}>{row.status}</span></td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => (selected = row)}
										aria-label="View"
									>
										<EyeIcon size={14} weight="bold" />
										<span>View</span>
									</button>
									{#if row.status !== 'spam'}
										<button
											class="btn btn--secondary btn--small"
											type="button"
											onclick={() => rowAction(row, 'mark_spam')}
											disabled={rowBusy[row.id] === 'mark_spam'}
											aria-label="Mark spam"
										>
											<ProhibitIcon size={14} weight="bold" />
											<span>Spam</span>
										</button>
									{/if}
									{#if row.status !== 'complete'}
										<button
											class="btn btn--secondary btn--small"
											type="button"
											onclick={() => rowAction(row, 'restore')}
											disabled={rowBusy[row.id] === 'restore'}
											aria-label="Restore"
										>
											<ArrowUUpLeftIcon size={14} weight="bold" />
											<span>Restore</span>
										</button>
									{/if}
									<button
										class="btn btn--danger btn--small"
										type="button"
										onclick={() => rowAction(row, 'delete')}
										disabled={rowBusy[row.id] === 'delete'}
										aria-label="Delete"
									>
										<TrashIcon size={14} weight="bold" />
										<span>Delete</span>
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

	{#if selected}
		<div
			class="drawer-backdrop"
			role="button"
			tabindex="-1"
			aria-label="Close inspector"
			onclick={() => (selected = null)}
			onkeydown={(e) => e.key === 'Escape' && (selected = null)}
		></div>
		<aside class="drawer" aria-label="Submission detail">
			<header class="drawer__header">
				<h2 class="drawer__title">Submission</h2>
				<button
					class="btn btn--secondary btn--small"
					type="button"
					onclick={() => (selected = null)}
				>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
				</button>
			</header>
			<dl class="meta">
				<dt>Id</dt><dd><code>{selected.id}</code></dd>
				<dt>Status</dt><dd><span class={statusPill(selected.status)}>{selected.status}</span></dd>
				<dt>Submitted</dt><dd>{new Date(selected.submitted_at).toLocaleString()}</dd>
				<dt>Subject</dt>
				<dd>
					{#if selected.subject_id}<code>{selected.subject_id}</code>{:else}—{/if}
				</dd>
				<dt>Anonymous</dt>
				<dd>
					{#if selected.anonymous_id}<code>{selected.anonymous_id}</code>{:else}—{/if}
				</dd>
				<dt>IP hash</dt><dd><code>{selected.ip_hash}</code></dd>
				<dt>User agent</dt><dd>{selected.user_agent || '—'}</dd>
				{#if selected.referrer}
					<dt>Referrer</dt><dd>{selected.referrer}</dd>
				{/if}
			</dl>
			<details open>
				<summary>Submitted data</summary>
				<pre class="json">{formatJson(selected.data_json)}</pre>
			</details>
			<details>
				<summary>Files</summary>
				<pre class="json">{formatJson(selected.files_json)}</pre>
			</details>
			<details>
				<summary>UTM</summary>
				<pre class="json">{formatJson(selected.utm)}</pre>
			</details>
			{#if selected.validation_errors}
				<details open>
					<summary>Validation errors</summary>
					<pre class="json">{formatJson(selected.validation_errors)}</pre>
				</details>
			{/if}
		</aside>
	{/if}
</div>

<style>
	.page { max-width: 80rem; padding: 0 0 3rem; }
	.page__header { display: flex; flex-wrap: wrap; gap: 1rem; align-items: flex-start; justify-content: space-between; margin-bottom: 1.25rem; }
	.page__title-row { display: flex; align-items: flex-start; gap: 0.85rem; color: var(--color-white); flex: 1; min-width: 0; }
	.page__copy { min-width: 0; }
	.page__actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }
	.eyebrow { display: inline-block; font-size: 0.6875rem; font-weight: 700; line-height: 1; letter-spacing: 0.08em; color: var(--color-grey-500); text-transform: uppercase; margin-bottom: 0.4rem; }
	.page__title { margin: 0; font-family: var(--font-heading); font-size: 1.5rem; font-weight: 700; color: var(--color-white); letter-spacing: -0.01em; line-height: 1.2; }
	.page__subtitle { margin: 0.35rem 0 0; font-size: 0.875rem; color: var(--color-grey-400); max-width: 42rem; line-height: 1.5; }
	.back-link { display: inline-flex; align-items: center; gap: 0.35rem; margin-top: 0.6rem; font-size: 0.75rem; color: var(--color-teal); text-decoration: none; font-weight: 600; }
	.back-link:hover { color: var(--color-teal-light); }

	.toast, .error { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; border-radius: var(--radius-lg); font-size: 0.875rem; margin-bottom: 1rem; }
	.toast { background: rgba(15, 164, 175, 0.12); border: 1px solid rgba(15, 164, 175, 0.25); color: #5eead4; }
	.error { background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5; }

	.filters { background: var(--color-navy-mid); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-xl); padding: 1.25rem; margin-bottom: 1.25rem; box-shadow: 0 1px 0 rgba(255, 255, 255, 0.03) inset, 0 12px 32px rgba(0, 0, 0, 0.18); }
	.filters__head { margin-bottom: 0.85rem; }
	.filters__eyebrow { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.6875rem; font-weight: 700; color: var(--color-grey-500); text-transform: uppercase; letter-spacing: 0.06em; }
	.filters__grid { display: grid; grid-template-columns: 1fr; gap: 0.85rem; }
	.filters__actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 1rem; align-items: center; }

	.bulkbar { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; padding: 0.75rem 1rem; margin-bottom: 1rem; background: rgba(15, 164, 175, 0.08); border: 1px solid rgba(15, 164, 175, 0.25); border-radius: var(--radius-lg); }
	.bulkbar__count { font-size: 0.8125rem; font-weight: 600; color: #5eead4; }
	.bulkbar__group { display: flex; align-items: center; gap: 0.5rem; }
	.field__input--inline { min-height: 2.25rem; padding: 0.4rem 0.65rem; }

	.field { display: flex; flex-direction: column; gap: 0.4rem; }
	.field__label { font-size: 0.75rem; color: var(--color-grey-300); font-weight: 500; }
	.field__input { min-height: 2.5rem; padding: 0.65rem 0.875rem; background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); color: var(--color-white); border-radius: var(--radius-lg); font-size: 0.875rem; width: 100%; font-family: inherit; color-scheme: dark; transition: border-color 150ms, box-shadow 150ms; }
	.field__input:focus { outline: none; border-color: var(--color-teal); box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15); }

	.btn { display: inline-flex; align-items: center; gap: 0.5rem; min-height: 2.5rem; padding: 0 0.875rem; border-radius: var(--radius-lg); font-size: 0.8125rem; font-weight: 600; border: 1px solid transparent; background: transparent; color: var(--color-grey-300); cursor: pointer; transition: background-color 150ms, border-color 150ms, color 150ms, box-shadow 150ms, transform 150ms; }
	.btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn--primary { background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94)); color: var(--color-white); box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45); }
	.btn--primary:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55); }
	.btn--secondary { background: rgba(255, 255, 255, 0.05); border-color: rgba(255, 255, 255, 0.1); color: var(--color-grey-200); }
	.btn--secondary:hover:not(:disabled) { background: rgba(255, 255, 255, 0.1); border-color: rgba(255, 255, 255, 0.18); color: var(--color-white); }
	.btn--danger { background: rgba(239, 68, 68, 0.1); color: #fca5a5; border-color: rgba(239, 68, 68, 0.3); }
	.btn--danger:hover:not(:disabled) { background: rgba(239, 68, 68, 0.18); }
	.btn--small { min-height: 2rem; padding: 0 0.65rem; font-size: 0.75rem; }

	.state { display: flex; align-items: center; justify-content: center; gap: 0.75rem; padding: 4rem 0; color: var(--color-grey-400); font-size: 0.875rem; }
	.state__spinner { width: 1.25rem; height: 1.25rem; border: 2px solid rgba(255, 255, 255, 0.1); border-top-color: var(--color-teal); border-radius: 50%; animation: spin 0.7s linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }

	.empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 3rem 1rem; background: var(--color-navy-mid); border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: var(--radius-xl); color: var(--color-grey-500); text-align: center; }
	.empty :global(svg) { color: var(--color-grey-500); }
	.empty__title { margin: 0.5rem 0 0; font-size: 1rem; font-weight: 600; color: var(--color-white); }
	.empty__sub { margin: 0; font-size: 0.875rem; color: var(--color-grey-400); }

	.card { background: var(--color-navy-mid); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-xl); box-shadow: 0 1px 0 rgba(255, 255, 255, 0.03) inset, 0 12px 32px rgba(0, 0, 0, 0.18); }
	.table-card { overflow: hidden; }
	.table-wrap { overflow-x: auto; }
	.table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
	.table th { text-align: left; font-weight: 700; color: var(--color-grey-500); font-size: 0.6875rem; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.75rem 1rem; background: rgba(255, 255, 255, 0.02); border-bottom: 1px solid rgba(255, 255, 255, 0.06); white-space: nowrap; }
	.table td { padding: 0.875rem 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.04); color: var(--color-grey-200); vertical-align: middle; }
	.table tbody tr:hover td { background: rgba(255, 255, 255, 0.02); }
	.table tbody tr:last-child td { border-bottom: none; }
	.table__actions-th { text-align: right; }
	.table__select-th { width: 2.5rem; }
	.table input[type='checkbox'] { accent-color: var(--color-teal); }
	.row-actions { display: flex; gap: 0.4rem; justify-content: flex-end; flex-wrap: wrap; }
	.ts { font-variant-numeric: tabular-nums; color: var(--color-grey-300); white-space: nowrap; }

	.pill { display: inline-flex; align-items: center; padding: 0.15rem 0.5rem; border-radius: var(--radius-full); font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
	.pill--success { background: rgba(15, 164, 175, 0.12); color: #5eead4; }
	.pill--warn { background: rgba(245, 158, 11, 0.12); color: #fcd34d; }
	.pill--danger { background: rgba(239, 68, 68, 0.12); color: #fca5a5; }
	.pill--neutral { background: rgba(255, 255, 255, 0.06); color: var(--color-grey-300); }

	.pager { display: flex; gap: 0.75rem; justify-content: center; align-items: center; margin-top: 1.25rem; flex-wrap: wrap; }
	.pager__info { font-size: 0.75rem; font-weight: 500; color: var(--color-grey-400); font-variant-numeric: tabular-nums; }

	.drawer-backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); z-index: 60; }
	.drawer { position: fixed; top: 0; right: 0; bottom: 0; width: min(640px, 92vw); background: var(--color-navy); border-left: 1px solid rgba(255, 255, 255, 0.08); padding: 1.5rem; overflow-y: auto; z-index: 70; box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3); }
	.drawer__header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
	.drawer__title { font-family: var(--font-heading); font-size: 1rem; font-weight: 600; color: var(--color-white); margin: 0; letter-spacing: -0.01em; }
	.meta { display: grid; grid-template-columns: 7rem 1fr; gap: 0.5rem 0.85rem; font-size: 0.875rem; color: var(--color-grey-200); margin-bottom: 1rem; }
	.meta dt { color: var(--color-grey-500); font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.meta dd { margin: 0; word-break: break-all; }
	.json { background: rgba(0, 0, 0, 0.3); padding: 0.85rem; border-radius: var(--radius-lg); font-size: 0.75rem; color: var(--color-grey-200); max-height: 40vh; overflow: auto; white-space: pre-wrap; word-break: break-all; }

	@media (min-width: 480px) {
		.filters__grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
	}
	@media (min-width: 768px) {
		.filters { padding: 1.75rem; border-radius: var(--radius-2xl); }
		.filters__grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
	}
</style>
