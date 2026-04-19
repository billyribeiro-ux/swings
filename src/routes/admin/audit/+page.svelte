<script lang="ts">
	import { onMount } from 'svelte';
	import Eye from 'phosphor-svelte/lib/Eye';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import DownloadSimple from 'phosphor-svelte/lib/DownloadSimple';
	import ArrowClockwise from 'phosphor-svelte/lib/ArrowClockwise';
	import CaretLeft from 'phosphor-svelte/lib/CaretLeft';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';
	import { auth } from '$lib/stores/auth.svelte';
	import { ApiError } from '$lib/api/client';
	import {
		auditLog,
		type AuditListEnvelope,
		type AuditListQuery,
		type AuditRow
	} from '$lib/api/admin-security';

	let envelope = $state<AuditListEnvelope | null>(null);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<AuditRow | null>(null);

	let filters = $state<AuditListQuery>({
		q: '',
		actor_id: '',
		action: '',
		target_kind: '',
		target_id: '',
		metadata_contains: '',
		from: '',
		to: '',
		limit: 50,
		offset: 0
	});

	function buildQuery(): AuditListQuery {
		const q: AuditListQuery = {
			limit: filters.limit ?? 50,
			offset: filters.offset ?? 0
		};
		if (filters.q?.trim()) q.q = filters.q.trim();
		if (filters.actor_id?.trim()) q.actor_id = filters.actor_id.trim();
		if (filters.action?.trim()) q.action = filters.action.trim();
		if (filters.target_kind?.trim()) q.target_kind = filters.target_kind.trim();
		if (filters.target_id?.trim()) q.target_id = filters.target_id.trim();
		if (filters.metadata_contains?.trim()) q.metadata_contains = filters.metadata_contains.trim();
		if (filters.from) q.from = new Date(filters.from).toISOString();
		if (filters.to) q.to = new Date(filters.to).toISOString();
		return q;
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await auditLog.list(buildQuery());
		} catch (e) {
			if (e instanceof ApiError) error = `${e.status}: ${e.message}`;
			else error = 'Failed to load audit log';
		} finally {
			loading = false;
		}
	}

	function applyFilters(e: Event) {
		e.preventDefault();
		filters.offset = 0;
		void refresh();
	}

	function clearFilters() {
		filters = {
			q: '',
			actor_id: '',
			action: '',
			target_kind: '',
			target_id: '',
			metadata_contains: '',
			from: '',
			to: '',
			limit: 50,
			offset: 0
		};
		void refresh();
	}

	function nextPage() {
		if (!envelope) return;
		if ((envelope.page ?? 1) >= (envelope.total_pages ?? 1)) return;
		filters.offset = (filters.offset ?? 0) + (filters.limit ?? 50);
		void refresh();
	}

	function prevPage() {
		if (!envelope) return;
		filters.offset = Math.max(0, (filters.offset ?? 0) - (filters.limit ?? 50));
		void refresh();
	}

	async function downloadCsv() {
		try {
			const url = auditLog.exportCsvUrl(buildQuery());
			const res = await fetch(url, {
				headers: {
					Authorization: auth.accessToken ? `Bearer ${auth.accessToken}` : ''
				}
			});
			if (!res.ok) {
				error = `Export failed (${res.status})`;
				return;
			}
			const blob = await res.blob();
			const a = document.createElement('a');
			const objUrl = URL.createObjectURL(blob);
			a.href = objUrl;
			a.download = `audit-${new Date().toISOString().slice(0, 10)}.csv`;
			document.body.appendChild(a);
			a.click();
			a.remove();
			URL.revokeObjectURL(objUrl);
		} catch {
			error = 'CSV export failed';
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
	<title>Audit log · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-audit-page">
	<header class="page__header">
		<div class="page__title-row">
			<Eye size={28} weight="duotone" />
			<h1 class="page__title">Audit log</h1>
		</div>
		<p class="page__subtitle">
			Searchable record of every privileged action. Free-text query runs over the indexed
			tsvector; <code>metadata_contains</code> accepts any JSON object and probes
			<code>metadata @&gt; &#123;...&#125;</code>. Date bounds are inclusive.
		</p>
	</header>

	{#if error}
		<div class="error" role="alert" data-testid="audit-error">{error}</div>
	{/if}

	<form class="filters" onsubmit={applyFilters}>
		<div class="filters__grid">
			<div class="field field--wide">
				<label class="field__label" for="audit-q">Free-text search</label>
				<div class="search-input">
					<MagnifyingGlass size={16} />
					<input
						id="audit-q"
						class="field__input"
						placeholder='e.g. "ban" OR "fraud"'
						bind:value={filters.q}
						data-testid="audit-q-input"
					/>
				</div>
			</div>
			<div class="field">
				<label class="field__label" for="audit-action">Action</label>
				<input
					id="audit-action"
					class="field__input"
					placeholder="admin.member.suspend"
					bind:value={filters.action}
					data-testid="audit-action-input"
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-actor">Actor user id</label>
				<input
					id="audit-actor"
					class="field__input"
					placeholder="UUID"
					bind:value={filters.actor_id}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-tkind">Target kind</label>
				<input
					id="audit-tkind"
					class="field__input"
					placeholder="user, order, …"
					bind:value={filters.target_kind}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-tid">Target id substring</label>
				<input
					id="audit-tid"
					class="field__input"
					placeholder="prefix or substring"
					bind:value={filters.target_id}
				/>
			</div>
			<div class="field field--wide">
				<label class="field__label" for="audit-meta">
					Metadata contains (JSON)
				</label>
				<input
					id="audit-meta"
					class="field__input field__input--mono"
					placeholder={'{"reason":"fraud"}'}
					bind:value={filters.metadata_contains}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-from">From</label>
				<input
					id="audit-from"
					type="datetime-local"
					class="field__input"
					bind:value={filters.from}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-to">To</label>
				<input
					id="audit-to"
					type="datetime-local"
					class="field__input"
					bind:value={filters.to}
				/>
			</div>
		</div>
		<div class="filters__actions">
			<button class="btn btn--primary" type="submit" data-testid="audit-apply">
				<MagnifyingGlass size={16} weight="bold" />
				Apply
			</button>
			<button class="btn btn--ghost" type="button" onclick={clearFilters}>Clear</button>
			<button class="btn btn--ghost" type="button" onclick={refresh} aria-label="Refresh">
				<ArrowClockwise size={16} weight="bold" />
				Refresh
			</button>
			<div class="filters__spacer"></div>
			<button class="btn btn--ghost" type="button" onclick={downloadCsv}>
				<DownloadSimple size={16} weight="bold" />
				Export CSV
			</button>
		</div>
	</form>

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if !envelope || envelope.data.length === 0}
		<p class="muted">No matching audit rows.</p>
	{:else}
		<div class="card table-wrap">
			<table class="table" data-testid="audit-table">
				<thead>
					<tr>
						<th>When</th>
						<th>Actor</th>
						<th>Action</th>
						<th>Target</th>
						<th>IP</th>
						<th aria-label="Inspect"></th>
					</tr>
				</thead>
				<tbody>
					{#each envelope.data as row (row.id)}
						<tr>
							<td title={row.created_at}>
								{new Date(row.created_at).toLocaleString()}
							</td>
							<td>
								<code title={row.actor_id}>{row.actor_id.slice(0, 8)}…</code>
								<span class="role-pill">{row.actor_role}</span>
							</td>
							<td><code class="action-key">{row.action}</code></td>
							<td>
								<span>{row.target_kind}</span>
								{#if row.target_id}
									<code class="target-id" title={row.target_id}>
										{row.target_id.length > 12
											? `${row.target_id.slice(0, 12)}…`
											: row.target_id}
									</code>
								{/if}
							</td>
							<td><code>{row.ip_address ?? '—'}</code></td>
							<td>
								<button
									class="btn btn--ghost btn--small"
									onclick={() => (selected = row)}
									data-testid="audit-inspect"
								>
									Inspect
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="pager">
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) <= 1}
				onclick={prevPage}
				aria-label="Previous page"
			>
				<CaretLeft size={16} />
				Prev
			</button>
			<span class="pager__info">
				Page {envelope.page} / {envelope.total_pages || 1} · {summaryRange()}
			</span>
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={nextPage}
				aria-label="Next page"
			>
				Next
				<CaretRight size={16} />
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
		<aside class="drawer" data-testid="audit-drawer" aria-label="Audit row detail">
			<header class="drawer__header">
				<h2 class="drawer__title">Audit entry</h2>
				<button
					class="btn btn--ghost btn--small"
					onclick={() => (selected = null)}
					aria-label="Close"
				>
					Close
				</button>
			</header>
			<dl class="drawer__meta">
				<dt>Action</dt>
				<dd><code>{selected.action}</code></dd>
				<dt>When</dt>
				<dd>{new Date(selected.created_at).toLocaleString()}</dd>
				<dt>Actor</dt>
				<dd>
					<code>{selected.actor_id}</code>
					<span class="role-pill">{selected.actor_role}</span>
				</dd>
				<dt>Target</dt>
				<dd>
					<code>{selected.target_kind}</code>
					{#if selected.target_id}
						· <code>{selected.target_id}</code>
					{/if}
				</dd>
				<dt>IP</dt>
				<dd><code>{selected.ip_address ?? '—'}</code></dd>
				<dt>User agent</dt>
				<dd>{selected.user_agent ?? '—'}</dd>
				<dt>Entry id</dt>
				<dd><code>{selected.id}</code></dd>
			</dl>
			<details open>
				<summary>Metadata</summary>
				<pre class="drawer__json">{formatJson(selected.metadata)}</pre>
			</details>
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
	.filters {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-4);
		margin-bottom: var(--space-4);
	}
	.filters__grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--space-3);
	}
	.filters__actions {
		display: flex;
		gap: var(--space-2);
		margin-top: var(--space-3);
		align-items: center;
	}
	.filters__spacer {
		flex: 1;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field--wide {
		grid-column: span 2;
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
	.field__input--mono {
		font-family: var(--font-mono, ui-monospace, monospace);
		font-size: 0.78rem;
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.search-input {
		position: relative;
	}
	.search-input :global(svg) {
		position: absolute;
		left: 0.7rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-400);
		pointer-events: none;
	}
	.search-input .field__input {
		padding-left: 2rem;
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
	.btn--small {
		padding: 0.25rem 0.6rem;
		font-size: var(--fs-xs);
	}
	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-3);
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
	.action-key {
		color: var(--color-teal-light);
	}
	.role-pill {
		display: inline-block;
		margin-left: 0.4rem;
		padding: 0.05rem 0.4rem;
		font-size: 0.65rem;
		text-transform: uppercase;
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
		border-radius: var(--radius-full);
		letter-spacing: 0.04em;
	}
	.target-id {
		display: inline-block;
		margin-left: 0.5rem;
		color: var(--color-grey-400);
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
		grid-template-columns: 7rem 1fr;
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
	.drawer__json {
		background: rgba(0, 0, 0, 0.3);
		padding: var(--space-3);
		border-radius: var(--radius-lg);
		font-size: 0.75rem;
		color: var(--color-grey-200);
		max-height: 50vh;
		overflow: auto;
		white-space: pre-wrap;
		word-break: break-all;
	}
</style>
