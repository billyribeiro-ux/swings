<!--
  Phase 2.3 — Consent log (read-only). Pulls from `consent_records`
  (CONSENT-03) when present; otherwise renders a friendly empty state with
  the pending-migration notice. Filterable by date range.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import ClipboardTextIcon from 'phosphor-svelte/lib/ClipboardTextIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import { ApiError } from '$lib/api/client';
	import { listLog, type ConsentLogResponse, type ConsentLogRow } from '$lib/api/admin-consent';

	let envelope = $state<ConsentLogResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<ConsentLogRow | null>(null);

	let limit = $state(50);
	let offset = $state(0);

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await listLog(limit, offset);
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load log';
		} finally {
			loading = false;
		}
	}

	function applyFilters(e: Event) {
		e.preventDefault();
		offset = 0;
		void refresh();
	}

	function nextPage() {
		if (!envelope) return;
		if (offset + limit >= envelope.total) return;
		offset += limit;
		void refresh();
	}
	function prevPage() {
		offset = Math.max(0, offset - limit);
		void refresh();
	}

	function formatJson(v: unknown): string {
		try {
			return JSON.stringify(v, null, 2);
		} catch {
			return String(v);
		}
	}

	function categoriesSummary(c: Record<string, boolean>): string {
		const accepted = Object.entries(c)
			.filter(([, v]) => v)
			.map(([k]) => k);
		if (accepted.length === 0) return 'none';
		return accepted.join(', ');
	}

	function actionPill(action: string): string {
		switch (action) {
			case 'grant':
			case 'accept_all':
				return 'pill pill--success';
			case 'reject_all':
				return 'pill pill--danger';
			case 'update':
				return 'pill pill--warn';
			default:
				return 'pill pill--neutral';
		}
	}

	const summaryRange = $derived.by(() => {
		if (!envelope) return '';
		const start = offset + 1;
		const end = offset + envelope.rows.length;
		return `${start}–${end} of ${envelope.total}`;
	});

	onMount(refresh);
</script>

<svelte:head>
	<title>Consent log · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-consent-log">
	<header class="page__header">
		<div class="page__title-row">
			<ClipboardTextIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Governance / Consent</span>
				<h1 class="page__title">Consent log</h1>
				<p class="page__subtitle">
					Append-only ledger of every consent record (accept-all, reject-all, granular
					updates). Used for proof-of-consent in DSAR responses and regulator audits.
				</p>
			</div>
		</div>
		<div class="page__actions">
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

	<form class="filters" onsubmit={applyFilters}>
		<header class="filters__head">
			<span class="filters__eyebrow">
				<FunnelIcon size={14} weight="bold" />
				Filters
			</span>
		</header>
		<div class="filters__grid">
			<div class="field">
				<label class="field__label" for="log-limit">Page size</label>
				<select id="log-limit" name="log-limit" class="field__input" bind:value={limit}>
					<option value={25}>25</option>
					<option value={50}>50</option>
					<option value={100}>100</option>
					<option value={250}>250</option>
				</select>
			</div>
		</div>
		<div class="filters__actions">
			<button class="btn btn--primary" type="submit">
				<MagnifyingGlassIcon size={16} weight="bold" />
				<span>Apply</span>
			</button>
		</div>
	</form>

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading consent log…</span>
		</div>
	{:else if envelope && !envelope.table_present}
		<div class="empty">
			<ClipboardTextIcon size={48} weight="duotone" />
			<p class="empty__title">Consent log table not provisioned</p>
			<p class="empty__sub">
				The CONSENT-03 migration hasn't run on this database yet. Apply migration
				<code>025_consent_log.sql</code> and refresh.
			</p>
		</div>
	{:else if !envelope || envelope.rows.length === 0}
		<div class="empty">
			<ClipboardTextIcon size={48} weight="duotone" />
			<p class="empty__title">No consent records yet</p>
			<p class="empty__sub">Records land as visitors interact with the consent banner.</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">When</th>
							<th scope="col">Subject</th>
							<th scope="col">Action</th>
							<th scope="col">Categories accepted</th>
							<th scope="col" class="table__actions-th" aria-label="Inspect"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.rows as row (row.id)}
							<tr>
								<td class="ts">{new Date(row.created_at).toLocaleString()}</td>
								<td>
									{#if row.subject_id}
										<code title={row.subject_id}
											>{row.subject_id.slice(0, 12)}…</code
										>
									{:else}
										<span class="muted">—</span>
									{/if}
								</td>
								<td><span class={actionPill(row.action)}>{row.action}</span></td>
								<td class="cats">{categoriesSummary(row.categories)}</td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => (selected = row)}
										aria-label="Inspect"
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
				type="button"
				disabled={offset === 0}
				onclick={prevPage}
			>
				<CaretLeftIcon size={16} weight="bold" />
				<span>Prev</span>
			</button>
			<span class="pager__info">{summaryRange}</span>
			<button
				class="btn btn--secondary"
				type="button"
				disabled={offset + limit >= envelope.total}
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
		<aside class="drawer" aria-label="Consent record">
			<header class="drawer__header">
				<h2 class="drawer__title">Consent record</h2>
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
				<dt>Id</dt>
				<dd><code>{selected.id}</code></dd>
				<dt>Subject</dt>
				<dd>
					{#if selected.subject_id}<code>{selected.subject_id}</code>{:else}—{/if}
				</dd>
				<dt>Action</dt>
				<dd><span class={actionPill(selected.action)}>{selected.action}</span></dd>
				<dt>Created</dt>
				<dd>{new Date(selected.created_at).toLocaleString()}</dd>
			</dl>
			<details open>
				<summary>Categories</summary>
				<pre class="json">{formatJson(selected.categories)}</pre>
			</details>
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
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
	}
	.empty__sub code {
		font-size: 0.85em;
		padding: 0.1em 0.35em;
		border-radius: 0.25rem;
		background: rgba(255, 255, 255, 0.06);
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
	.muted {
		color: var(--color-grey-500);
	}
	.cats {
		font-size: 0.8125rem;
		color: var(--color-grey-300);
		max-width: 36ch;
		overflow: hidden;
		text-overflow: ellipsis;
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
	.meta {
		display: grid;
		grid-template-columns: 6rem 1fr;
		gap: 0.5rem 0.85rem;
		font-size: 0.875rem;
		color: var(--color-grey-200);
		margin-bottom: 1rem;
	}
	.meta dt {
		color: var(--color-grey-500);
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.meta dd {
		margin: 0;
		word-break: break-all;
	}
	.json {
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
</style>
