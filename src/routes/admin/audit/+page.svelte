<script lang="ts">
	import { onMount } from 'svelte';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import DownloadSimpleIcon from 'phosphor-svelte/lib/DownloadSimpleIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import { ApiError } from '$lib/api/client';
	import { toast } from '$lib/stores/toast.svelte';
	import { Drawer } from '$lib/components/shared';
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
			// BFF (Phase 1.3): cookie-based auth — no Bearer header needed.
			const res = await fetch(url, { credentials: 'include' });
			if (!res.ok) {
				toast.error('Audit export failed', { description: `Server returned ${res.status}` });
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
			toast.success('Audit log exported');
		} catch (e) {
			toast.error('Audit export failed', {
				description: e instanceof Error ? e.message : undefined
			});
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
			<EyeIcon size={28} weight="duotone" />
			<div class="page__copy">
				<h1 class="page__title">Audit log</h1>
				<p class="page__subtitle">
					Searchable record of every privileged action. Free-text query runs over the indexed
					tsvector; <code>metadata_contains</code> accepts a JSON object and probes
					<code>metadata @&gt; &#123;...&#125;</code>.
				</p>
			</div>
		</div>
	</header>

	{#if error}
		<div class="error" role="alert" data-testid="audit-error">{error}</div>
	{/if}

	<form class="filters" onsubmit={applyFilters}>
		<header class="filters__head">
			<span class="filters__eyebrow">
				<FunnelIcon size={14} weight="bold" />
				Filters
			</span>
		</header>
		<div class="filters__grid">
			<div class="field field--wide">
				<label class="field__label" for="audit-q">Free-text search</label>
				<div class="search-input">
					<MagnifyingGlassIcon size={16} />
					<input
						id="audit-q"
						name="audit-q"
						class="field__input field__input--search"
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
					name="audit-action"
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
					name="audit-actor"
					class="field__input"
					placeholder="UUID"
					bind:value={filters.actor_id}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-tkind">Target kind</label>
				<input
					id="audit-tkind"
					name="audit-tkind"
					class="field__input"
					placeholder="user, order, …"
					bind:value={filters.target_kind}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-tid">Target id substring</label>
				<input
					id="audit-tid"
					name="audit-tid"
					class="field__input"
					placeholder="prefix or substring"
					bind:value={filters.target_id}
				/>
			</div>
			<div class="field field--wide">
				<label class="field__label" for="audit-meta">Metadata contains (JSON)</label>
				<input
					id="audit-meta"
					name="audit-meta"
					class="field__input field__input--mono"
					placeholder={'{"reason":"fraud"}'}
					bind:value={filters.metadata_contains}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-from">From</label>
				<input
					id="audit-from"
					name="audit-from"
					type="datetime-local"
					class="field__input"
					bind:value={filters.from}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="audit-to">To</label>
				<input
					id="audit-to"
					name="audit-to"
					type="datetime-local"
					class="field__input"
					bind:value={filters.to}
				/>
			</div>
		</div>
		<div class="filters__actions">
			<button class="btn btn--primary" type="submit" data-testid="audit-apply">
				<MagnifyingGlassIcon size={16} weight="bold" />
				<span>Apply</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={clearFilters}>
				<XIcon size={16} weight="bold" />
				<span>Clear</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={refresh}>
				<ArrowsClockwiseIcon size={16} weight="bold" />
				<span>Refresh</span>
			</button>
			<div class="filters__spacer"></div>
			<button class="btn btn--secondary" type="button" onclick={downloadCsv}>
				<DownloadSimpleIcon size={16} weight="bold" />
				<span>Export CSV</span>
			</button>
		</div>
	</form>

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading audit rows…</span>
		</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty">
			<EyeIcon size={48} weight="duotone" />
			<p class="empty__title">No matching audit rows</p>
			<p class="empty__sub">Adjust filters or widen the date range.</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table" data-testid="audit-table">
					<thead>
						<tr>
							<th scope="col">When</th>
							<th scope="col">Actor</th>
							<th scope="col">Action</th>
							<th scope="col">Target</th>
							<th scope="col">IP</th>
							<th scope="col" class="table__actions-th" aria-label="Inspect"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.data as row (row.id)}
							<tr>
								<td class="table__ts" title={row.created_at}>
									{new Date(row.created_at).toLocaleString()}
								</td>
								<td>
									<code title={row.actor_id}>{row.actor_id.slice(0, 8)}…</code>
									<span class="pill pill--neutral">{row.actor_role}</span>
								</td>
								<td><code class="action-key">{row.action}</code></td>
								<td>
									<span class="target-kind">{row.target_kind}</span>
									{#if row.target_id}
										<code class="target-id" title={row.target_id}>
											{row.target_id.length > 12
												? `${row.target_id.slice(0, 12)}…`
												: row.target_id}
										</code>
									{/if}
								</td>
								<td><code>{row.ip_address ?? '—'}</code></td>
								<td class="table__actions">
									<button
										class="btn btn--secondary btn--small"
										onclick={() => (selected = row)}
										data-testid="audit-inspect"
										aria-label="Inspect row"
									>
										<EyeIcon size={14} weight="bold" />
										<span>Inspect</span>
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
				disabled={(envelope.page ?? 1) <= 1}
				onclick={prevPage}
				aria-label="Previous page"
			>
				<CaretLeftIcon size={16} weight="bold" />
				<span>Prev</span>
			</button>
			<span class="pager__info">
				Page {envelope.page} / {envelope.total_pages || 1} · {summaryRange()}
			</span>
			<button
				class="btn btn--secondary"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={nextPage}
				aria-label="Next page"
			>
				<span>Next</span>
				<CaretRightIcon size={16} weight="bold" />
			</button>
		</div>
	{/if}

	<Drawer
		open={selected !== null}
		onclose={() => (selected = null)}
		title="Audit entry"
		size="lg"
	>
		{#if selected}
			<div data-testid="audit-drawer">
				<dl class="drawer__meta">
					<dt>Action</dt>
					<dd><code>{selected.action}</code></dd>
					<dt>When</dt>
					<dd>{new Date(selected.created_at).toLocaleString()}</dd>
					<dt>Actor</dt>
					<dd>
						<code>{selected.actor_id}</code>
						<span class="pill pill--neutral">{selected.actor_role}</span>
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
			</div>
		{/if}
		{#snippet footer()}
			<button class="btn btn--secondary" type="button" onclick={() => (selected = null)}>
				<XIcon size={14} weight="bold" />
				<span>Close</span>
			</button>
		{/snippet}
	</Drawer>
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
		line-height: 1.2;
	}
	.page__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
	}
	.page__subtitle code {
		font-size: 0.85em;
		padding: 0.1em 0.35em;
		border-radius: 0.25rem;
		background: rgba(255, 255, 255, 0.06);
	}

	.error {
		padding: 0.85rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		margin-bottom: 1rem;
	}

	.filters {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
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
	.filters__spacer {
		flex: 1;
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
	.field__input--mono {
		font-family: var(--font-mono);
		font-size: 0.78rem;
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}
	.search-input {
		position: relative;
	}
	.search-input :global(svg) {
		position: absolute;
		left: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-400);
		pointer-events: none;
	}
	.field__input--search {
		padding-left: 2.25rem;
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
	.btn--small {
		min-height: 2.5rem;
		padding: 0 0.65rem;
		font-size: 0.75rem;
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
	.table__actions {
		text-align: right;
	}
	.action-key {
		color: var(--color-teal-light);
	}
	.target-kind {
		color: var(--color-grey-200);
	}
	.target-id {
		display: inline-block;
		margin-left: 0.5rem;
		color: var(--color-grey-400);
	}

	.pill {
		display: inline-flex;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-left: 0.4rem;
	}
	.pill--neutral {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
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

	.drawer__meta {
		display: grid;
		grid-template-columns: 7rem 1fr;
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
	.drawer__json {
		background: rgba(0, 0, 0, 0.3);
		padding: 0.85rem;
		border-radius: var(--radius-2xl);
		font-size: 0.75rem;
		color: var(--color-grey-200);
		max-height: 50vh;
		overflow: auto;
		white-space: pre-wrap;
		word-break: break-all;
	}

	@media (min-width: 480px) {
		.filters__grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (min-width: 768px) {
		.filters {
			padding: 1.75rem;
			border-radius: var(--radius-2xl);
		}
		.filters__grid {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}
		.field--wide {
			grid-column: span 2;
		}
	}
	@media (min-width: 1024px) {
		.filters__grid {
			grid-template-columns: repeat(4, minmax(0, 1fr));
		}
	}
</style>
