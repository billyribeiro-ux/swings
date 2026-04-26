<!--
  Phase 2.2 — Outbox events admin. Lists `outbox_events` with status filter,
  per-row retry, and a drilldown drawer that shows the full event JSON +
  attempts history (last_error). Retry sets status back to `pending` and
  schedules the row for immediate re-dispatch.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import StackIcon from 'phosphor-svelte/lib/StackIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import { ApiError } from '$lib/api/client';
	import { toast } from '$lib/stores/toast.svelte';
	import {
		outbox,
		type OutboxListQuery,
		type OutboxRowDto,
		type OutboxStatusFilter,
		type PaginatedOutboxResponse
	} from '$lib/api/admin-outbox';

	let envelope = $state<PaginatedOutboxResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<OutboxRowDto | null>(null);
	let retryingId = $state<string | null>(null);

	let filters = $state<OutboxListQuery>({ status: '', page: 1, per_page: 25 });

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await outbox.list(filters);
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load outbox';
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
		filters = { status: '', page: 1, per_page: 25 };
		void refresh();
	}

	async function retryRow(row: OutboxRowDto) {
		retryingId = row.id;
		error = '';
		try {
			await outbox.retry(row.id);
			toast.success(`Re-queued event ${row.id.slice(0, 8)}…`);
			await refresh();
		} catch (e) {
			toast.error('Retry failed', {
				description: e instanceof ApiError ? `${e.status}: ${e.message}` : undefined
			});
		} finally {
			retryingId = null;
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
			case 'delivered':
				return 'pill pill--success';
			case 'pending':
			case 'in_flight':
				return 'pill pill--warn';
			case 'failed':
			case 'dead_letter':
				return 'pill pill--danger';
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

	const summaryRange = $derived.by(() => {
		if (!envelope) return '';
		const start = ((envelope.page ?? 1) - 1) * (envelope.per_page ?? 25) + 1;
		const end = start + envelope.data.length - 1;
		return `${start}–${end} of ${envelope.total}`;
	});

	const statusOptions: ReadonlyArray<{ value: OutboxStatusFilter | ''; label: string }> = [
		{ value: '', label: 'Any' },
		{ value: 'pending', label: 'Pending' },
		{ value: 'in_flight', label: 'In flight' },
		{ value: 'delivered', label: 'Delivered' },
		{ value: 'failed', label: 'Failed' },
		{ value: 'dead_letter', label: 'Dead letter' }
	];

	onMount(refresh);
</script>

<svelte:head>
	<title>Outbox · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-outbox">
	<header class="page__header">
		<div class="page__title-row">
			<StackIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Operations</span>
				<h1 class="page__title">Outbox events</h1>
				<p class="page__subtitle">
					Transactional outbox (FDN-04). Each row is one domain event awaiting fan-out by the
					background dispatcher. Retry pulls a failed or dead-letter row back to <code>pending</code>
					and schedules it for the next worker tick.
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
				<label class="field__label" for="ob-status">Status</label>
				<select
					id="ob-status"
					name="ob-status"
					class="field__input"
					bind:value={filters.status}
				>
					{#each statusOptions as opt (opt.value)}
						<option value={opt.value}>{opt.label}</option>
					{/each}
				</select>
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

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading outbox…</span>
		</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty">
			<StackIcon size={48} weight="duotone" />
			<p class="empty__title">No events match</p>
			<p class="empty__sub">An empty outbox usually means the dispatcher is healthy.</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Id</th>
							<th scope="col">Event type</th>
							<th scope="col">Status</th>
							<th scope="col">Attempts</th>
							<th scope="col">Next attempt</th>
							<th scope="col">Last error</th>
							<th scope="col" class="table__actions-th" aria-label="Actions"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.data as row (row.id)}
							<tr>
								<td><code title={row.id}>{row.id.slice(0, 8)}…</code></td>
								<td>
									<code class="key">{row.event_type}</code>
									<div class="muted-inline">{row.aggregate_type}</div>
								</td>
								<td><span class={statusPill(String(row.status))}>{row.status}</span></td>
								<td class="num">{row.attempts} / {row.max_attempts}</td>
								<td class="ts">{new Date(row.next_attempt_at).toLocaleString()}</td>
								<td class="error-cell" title={row.last_error ?? ''}>
									{row.last_error ?? '—'}
								</td>
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
									{#if String(row.status) !== 'delivered'}
										<button
											class="btn btn--primary btn--small"
											type="button"
											onclick={() => retryRow(row)}
											disabled={retryingId === row.id}
											aria-label="Retry"
										>
											<ArrowsClockwiseIcon size={14} weight="bold" />
											<span>{retryingId === row.id ? 'Retrying…' : 'Retry'}</span>
										</button>
									{/if}
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
		<aside class="drawer" aria-label="Outbox event detail">
			<header class="drawer__header">
				<h2 class="drawer__title">Outbox event</h2>
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
				<dt>Event type</dt><dd><code>{selected.event_type}</code></dd>
				<dt>Aggregate</dt>
				<dd>
					<code>{selected.aggregate_type}</code> · <code>{selected.aggregate_id}</code>
				</dd>
				<dt>Status</dt><dd><span class={statusPill(String(selected.status))}>{selected.status}</span></dd>
				<dt>Attempts</dt><dd class="num">{selected.attempts} / {selected.max_attempts}</dd>
				<dt>Next attempt</dt><dd>{new Date(selected.next_attempt_at).toLocaleString()}</dd>
				<dt>Created</dt><dd>{new Date(selected.created_at).toLocaleString()}</dd>
				<dt>Updated</dt><dd>{new Date(selected.updated_at).toLocaleString()}</dd>
				{#if selected.last_error}
					<dt>Last error</dt><dd class="err">{selected.last_error}</dd>
				{/if}
			</dl>
			<details open>
				<summary>Payload</summary>
				<pre class="json">{formatJson(selected.payload)}</pre>
			</details>
			<details>
				<summary>Headers</summary>
				<pre class="json">{formatJson(selected.headers)}</pre>
			</details>
			{#if String(selected.status) !== 'delivered'}
				<div class="form__actions">
					<button
						class="btn btn--primary"
						type="button"
						onclick={() => retryRow(selected!)}
						disabled={retryingId === selected.id}
					>
						<ArrowsClockwiseIcon size={16} weight="bold" />
						<span>{retryingId === selected.id ? 'Retrying…' : 'Retry now'}</span>
					</button>
				</div>
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
	.page__subtitle code { font-size: 0.85em; padding: 0.1em 0.35em; border-radius: 0.25rem; background: rgba(255, 255, 255, 0.06); }

	.error { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; border-radius: var(--radius-2xl); font-size: 0.875rem; margin-bottom: 1rem; }
	.error { background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5; }

	.filters { background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-2xl); padding: 1.25rem; margin-bottom: 1.25rem; box-shadow: 0 1px 0 rgba(255, 255, 255, 0.03) inset, 0 12px 32px rgba(0, 0, 0, 0.18); }
	.filters__head { margin-bottom: 0.85rem; }
	.filters__eyebrow { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.6875rem; font-weight: 700; color: var(--color-grey-500); text-transform: uppercase; letter-spacing: 0.06em; }
	.filters__grid { display: grid; grid-template-columns: 1fr; gap: 0.85rem; }
	.filters__actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 1rem; align-items: center; }

	.field { display: flex; flex-direction: column; gap: 0.4rem; }
	.field__label { font-size: 0.75rem; color: var(--color-grey-300); font-weight: 500; }
	.field__input { min-height: 3rem; padding: 0 1.25rem; background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); color: var(--color-white); border-radius: var(--radius-2xl); font-size: 0.875rem; width: 100%; font-family: inherit; color-scheme: dark; transition: border-color 150ms, box-shadow 150ms; }
	.field__input:focus { outline: none; border-color: var(--color-teal); box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15); }

	.btn { display: inline-flex; align-items: center; gap: 0.5rem; min-height: 3rem; padding: 0 1.25rem; border-radius: var(--radius-2xl); font-size: 0.8125rem; font-weight: 600; border: 1px solid transparent; background: transparent; color: var(--color-grey-300); cursor: pointer; transition: background-color 150ms, border-color 150ms, color 150ms, box-shadow 150ms, transform 150ms; }
	.btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn--primary { background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94)); color: var(--color-white); box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45); }
	.btn--primary:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55); }
	.btn--secondary { background: rgba(255, 255, 255, 0.05); border-color: rgba(255, 255, 255, 0.1); color: var(--color-grey-200); }
	.btn--secondary:hover:not(:disabled) { background: rgba(255, 255, 255, 0.1); border-color: rgba(255, 255, 255, 0.18); color: var(--color-white); }
	.btn--small { min-height: 2.5rem; padding: 0 0.65rem; font-size: 0.75rem; }

	.state { display: flex; align-items: center; justify-content: center; gap: 0.75rem; padding: 4rem 0; color: var(--color-grey-400); font-size: 0.875rem; }
	.state__spinner { width: 1.25rem; height: 1.25rem; border: 2px solid rgba(255, 255, 255, 0.1); border-top-color: var(--color-teal); border-radius: 50%; animation: spin 0.7s linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }

	.empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 3rem 1rem; background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px); border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: var(--radius-2xl); color: var(--color-grey-500); text-align: center; }
	.empty :global(svg) { color: var(--color-grey-500); }
	.empty__title { margin: 0.5rem 0 0; font-size: 1rem; font-weight: 600; color: var(--color-white); }
	.empty__sub { margin: 0; font-size: 0.875rem; color: var(--color-grey-400); }

	.card { background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-2xl); box-shadow: 0 1px 0 rgba(255, 255, 255, 0.03) inset, 0 12px 32px rgba(0, 0, 0, 0.18); }
	.table-card { overflow: hidden; }
	.table-wrap { overflow-x: auto; }
	.table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
	.table th { text-align: left; font-weight: 700; color: var(--color-grey-500); font-size: 0.6875rem; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.75rem 1rem; background: rgba(255, 255, 255, 0.02); border-bottom: 1px solid rgba(255, 255, 255, 0.06); white-space: nowrap; }
	.table td { padding: 0.875rem 1rem; border-bottom: 1px solid rgba(255, 255, 255, 0.04); color: var(--color-grey-200); vertical-align: middle; }
	.table tbody tr:hover td { background: rgba(255, 255, 255, 0.02); }
	.table tbody tr:last-child td { border-bottom: none; }
	.table__actions-th { text-align: right; }
	.row-actions { display: flex; gap: 0.4rem; justify-content: flex-end; flex-wrap: wrap; }
	.ts { font-variant-numeric: tabular-nums; color: var(--color-grey-300); white-space: nowrap; }
	.num { font-variant-numeric: tabular-nums; text-align: right; color: var(--color-grey-300); }
	.key { color: var(--color-teal-light); }
	.muted-inline { color: var(--color-grey-500); font-size: 0.75rem; margin-top: 0.15rem; }
	.error-cell { color: #fca5a5; max-width: 22ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.err { color: #fca5a5; word-break: break-word; }

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
	.meta { display: grid; grid-template-columns: 8rem 1fr; gap: 0.5rem 0.85rem; font-size: 0.875rem; color: var(--color-grey-200); margin-bottom: 1rem; }
	.meta dt { color: var(--color-grey-500); font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.meta dd { margin: 0; word-break: break-all; }
	.json { background: rgba(0, 0, 0, 0.3); padding: 0.85rem; border-radius: var(--radius-2xl); font-size: 0.75rem; color: var(--color-grey-200); max-height: 40vh; overflow: auto; white-space: pre-wrap; word-break: break-all; }
	.form__actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 1rem; }

	@media (min-width: 480px) {
		.filters__grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
	}
	@media (min-width: 768px) {
		.filters { padding: 1.75rem; border-radius: var(--radius-2xl); }
	}
</style>
