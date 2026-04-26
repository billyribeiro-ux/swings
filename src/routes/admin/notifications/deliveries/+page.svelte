<!--
  Phase 2.1 — Notification delivery log (read-only). Filterable by
  template_key (sent as `key`-derived `user_id` in the future), recipient
  user id, status, and date range. The drawer surfaces the full delivery
  payload + provider response.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import PaperPlaneTiltIcon from 'phosphor-svelte/lib/PaperPlaneTiltIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import { ApiError } from '$lib/api/client';
	import {
		deliveries,
		type DeliveryListQuery,
		type DeliveryRow,
		type PaginatedDeliveriesResponse
	} from '$lib/api/admin-notifications';

	let envelope = $state<PaginatedDeliveriesResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<DeliveryRow | null>(null);

	let filters = $state<DeliveryListQuery>({
		status: '',
		user_id: '',
		page: 1,
		per_page: 25
	});

	function buildQuery(): DeliveryListQuery {
		const q: DeliveryListQuery = {
			page: filters.page ?? 1,
			per_page: filters.per_page ?? 25
		};
		if (filters.status?.trim()) q.status = filters.status.trim();
		if (filters.user_id?.trim()) q.user_id = filters.user_id.trim();
		return q;
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await deliveries.list(buildQuery());
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load deliveries';
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
		filters = { status: '', user_id: '', page: 1, per_page: 25 };
		void refresh();
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
			case 'sent':
			case 'delivered':
				return 'pill pill--success';
			case 'queued':
			case 'processing':
				return 'pill pill--warn';
			case 'failed':
			case 'bounced':
			case 'rejected':
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

	onMount(refresh);
</script>

<svelte:head>
	<title>Delivery log · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-notifications-deliveries">
	<header class="page__header">
		<div class="page__title-row">
			<PaperPlaneTiltIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Operations / Notifications</span>
				<h1 class="page__title">Delivery log</h1>
				<p class="page__subtitle">
					Every send the platform has attempted, with status, provider id, and rendered subject. Use
					this to triage "did the user get the email?" tickets.
				</p>
			</div>
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
				<label class="field__label" for="del-status">Status</label>
				<select
					id="del-status"
					name="del-status"
					class="field__input"
					bind:value={filters.status}
				>
					<option value="">Any</option>
					<option value="queued">Queued</option>
					<option value="processing">Processing</option>
					<option value="sent">Sent</option>
					<option value="delivered">Delivered</option>
					<option value="bounced">Bounced</option>
					<option value="failed">Failed</option>
					<option value="rejected">Rejected</option>
				</select>
			</div>
			<div class="field field--wide">
				<label class="field__label" for="del-user">Recipient user id</label>
				<input
					id="del-user"
					name="del-user"
					class="field__input"
					placeholder="UUID"
					bind:value={filters.user_id}
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
			<button class="btn btn--secondary" type="button" onclick={() => void refresh()}>
				<ArrowsClockwiseIcon size={16} weight="bold" />
				<span>Refresh</span>
			</button>
		</div>
	</form>

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading deliveries…</span>
		</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty">
			<PaperPlaneTiltIcon size={48} weight="duotone" />
			<p class="empty__title">No deliveries match</p>
			<p class="empty__sub">Widen the filters or wait for a send to land.</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">When</th>
							<th scope="col">Template</th>
							<th scope="col">Recipient</th>
							<th scope="col">Status</th>
							<th scope="col">Provider id</th>
							<th scope="col" class="table__actions-th" aria-label="Inspect"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.data as row (row.id)}
							<tr>
								<td class="ts" title={row.created_at}>
									{new Date(row.created_at).toLocaleString()}
								</td>
								<td>
									<code class="key">{row.template_key}</code>
									<span class="pill pill--neutral">{row.channel}</span>
								</td>
								<td>
									{#if row.user_id}
										<code title={row.user_id}>{row.user_id.slice(0, 8)}…</code>
									{:else if row.anonymous_email}
										<code title={row.anonymous_email}>{row.anonymous_email}</code>
									{:else}
										<span class="muted">—</span>
									{/if}
								</td>
								<td><span class={statusPill(row.status)}>{row.status}</span></td>
								<td>
									{#if row.provider_id}
										<code title={row.provider_id}>{row.provider_id.slice(0, 14)}…</code>
									{:else}
										<span class="muted">—</span>
									{/if}
								</td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => (selected = row)}
										aria-label="Inspect delivery"
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
		<aside class="drawer" aria-label="Delivery detail">
			<header class="drawer__header">
				<h2 class="drawer__title">Delivery</h2>
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
				<dt>Template</dt>
				<dd>
					<code>{selected.template_key}</code> · <span class="muted-inline">{selected.channel}</span>
				</dd>
				<dt>Status</dt>
				<dd><span class={statusPill(selected.status)}>{selected.status}</span></dd>
				<dt>Recipient</dt>
				<dd>
					{#if selected.user_id}
						<code>{selected.user_id}</code>
					{:else if selected.anonymous_email}
						<code>{selected.anonymous_email}</code>
					{:else}
						—
					{/if}
				</dd>
				<dt>Subject</dt>
				<dd>{selected.subject ?? '—'}</dd>
				<dt>Provider id</dt>
				<dd>
					{#if selected.provider_id}
						<code>{selected.provider_id}</code>
					{:else}
						—
					{/if}
				</dd>
				<dt>Created</dt>
				<dd>{new Date(selected.created_at).toLocaleString()}</dd>
				<dt>Updated</dt>
				<dd>{new Date(selected.updated_at).toLocaleString()}</dd>
			</dl>
			<details open>
				<summary>Rendered body</summary>
				<pre class="json">{selected.rendered_body}</pre>
			</details>
			<details>
				<summary>Provider metadata</summary>
				<pre class="json">{formatJson(selected.metadata)}</pre>
			</details>
			{#if selected.status === 'failed' || selected.status === 'bounced'}
				<p class="hint">
					Re-sending failed deliveries currently goes through the source channel — for transactional
					sends, retrigger the originating action; for marketing broadcasts, fix the suppression
					list and re-broadcast.
				</p>
			{/if}
		</aside>
	{/if}
</div>

<style>
	.page { max-width: 80rem; padding: 0 0 3rem; }
	.page__header { margin-bottom: 1.25rem; }
	.page__title-row { display: flex; align-items: flex-start; gap: 0.85rem; color: var(--color-white); }
	.page__copy { min-width: 0; }
	.eyebrow { display: inline-block; font-size: 0.6875rem; font-weight: 700; line-height: 1; letter-spacing: 0.08em; color: var(--color-grey-500); text-transform: uppercase; margin-bottom: 0.4rem; }
	.page__title { margin: 0; font-family: var(--font-heading); font-size: 1.5rem; font-weight: 700; color: var(--color-white); letter-spacing: -0.01em; line-height: 1.2; }
	.page__subtitle { margin: 0.35rem 0 0; font-size: 0.875rem; color: var(--color-grey-400); max-width: 42rem; line-height: 1.5; }

	.error { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; border-radius: var(--radius-lg); font-size: 0.875rem; margin-bottom: 1rem; background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5; }

	.filters { background: var(--color-navy-mid); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-xl); padding: 1.25rem; margin-bottom: 1.25rem; box-shadow: 0 1px 0 rgba(255, 255, 255, 0.03) inset, 0 12px 32px rgba(0, 0, 0, 0.18); }
	.filters__head { margin-bottom: 0.85rem; }
	.filters__eyebrow { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.6875rem; font-weight: 700; color: var(--color-grey-500); text-transform: uppercase; letter-spacing: 0.06em; }
	.filters__grid { display: grid; grid-template-columns: 1fr; gap: 0.85rem; }
	.filters__actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 1rem; align-items: center; }

	.field { display: flex; flex-direction: column; gap: 0.4rem; }
	.field--wide { grid-column: span 1; }
	.field__label { font-size: 0.75rem; color: var(--color-grey-300); font-weight: 500; }
	.field__input { min-height: 2.5rem; padding: 0.65rem 0.875rem; background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); color: var(--color-white); border-radius: var(--radius-lg); font-size: 0.875rem; width: 100%; font-family: inherit; color-scheme: dark; transition: border-color 150ms, box-shadow 150ms; }
	.field__input::placeholder { color: var(--color-grey-500); }
	.field__input:focus { outline: none; border-color: var(--color-teal); box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15); }

	.btn { display: inline-flex; align-items: center; gap: 0.5rem; min-height: 2.5rem; padding: 0 0.875rem; border-radius: var(--radius-lg); font-size: 0.8125rem; font-weight: 600; border: 1px solid transparent; background: transparent; color: var(--color-grey-300); cursor: pointer; transition: background-color 150ms, border-color 150ms, color 150ms, box-shadow 150ms, transform 150ms; }
	.btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn--primary { background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94)); color: var(--color-white); box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45); }
	.btn--primary:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55); }
	.btn--secondary { background: rgba(255, 255, 255, 0.05); border-color: rgba(255, 255, 255, 0.1); color: var(--color-grey-200); }
	.btn--secondary:hover:not(:disabled) { background: rgba(255, 255, 255, 0.1); border-color: rgba(255, 255, 255, 0.18); color: var(--color-white); }
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
	.row-actions { display: flex; gap: 0.4rem; justify-content: flex-end; flex-wrap: wrap; }
	.ts { font-variant-numeric: tabular-nums; color: var(--color-grey-300); white-space: nowrap; }
	.key { color: var(--color-teal-light); margin-right: 0.4rem; }
	.muted, .muted-inline { color: var(--color-grey-500); }

	.pill { display: inline-flex; align-items: center; padding: 0.15rem 0.5rem; border-radius: var(--radius-full); font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; margin-left: 0.4rem; }
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
	.json { background: rgba(0, 0, 0, 0.3); padding: 0.85rem; border-radius: var(--radius-lg); font-size: 0.75rem; color: var(--color-grey-200); max-height: 50vh; overflow: auto; white-space: pre-wrap; word-break: break-all; }
	.hint { margin: 0.85rem 0 0; font-size: 0.75rem; color: var(--color-grey-500); line-height: 1.45; }

	@media (min-width: 480px) {
		.filters__grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
	}
	@media (min-width: 768px) {
		.filters { padding: 1.75rem; border-radius: var(--radius-2xl); }
		.filters__grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
		.field--wide { grid-column: span 2; }
	}
</style>
