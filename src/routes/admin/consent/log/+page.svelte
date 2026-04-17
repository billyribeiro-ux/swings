<!--
  CONSENT-07 — consent log viewer.

  Read-only. Pulls from `consent_records` (CONSENT-03) when that table is
  present; otherwise shows an empty state with the pending-migration notice
  so the admin knows the log isn't broken — it just hasn't been migrated yet
  (CONSENT-03 lives in a sibling worktree).

  Also renders the 100 most recent integrity anchors below the log so
  auditors have a one-page surface for recomputing the hash chain.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/shared';
	import {
		listLog,
		listIntegrity,
		type ConsentLogRow,
		type IntegrityAnchor
	} from '$lib/api/admin-consent';

	let rows = $state<ConsentLogRow[]>([]);
	let tablePresent = $state(false);
	let total = $state(0);
	let loading = $state(true);
	let errorMsg = $state<string | null>(null);

	let anchors = $state<IntegrityAnchor[]>([]);

	let limit = $state(50);
	let offset = $state(0);

	async function refresh() {
		loading = true;
		errorMsg = null;
		try {
			const [log, integ] = await Promise.all([listLog(limit, offset), listIntegrity()]);
			rows = [...log.rows];
			tablePresent = log.table_present;
			total = log.total;
			anchors = [...integ];
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	onMount(() => void refresh());

	function prev() {
		offset = Math.max(0, offset - limit);
		void refresh();
	}
	function next() {
		offset = Math.min(total - 1, offset + limit);
		void refresh();
	}
</script>

<svelte:head>
	<title>Log · Consent · Admin</title>
</svelte:head>

<header class="head">
	<h1>Consent log</h1>
</header>

{#if errorMsg}<div class="error">{errorMsg}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if !tablePresent}
	<section class="empty">
		<h2>Log table not present</h2>
		<p>
			The <code>consent_records</code> table is not in this database. Apply the CONSENT-03
			migration to enable the log. Integrity anchors below will populate once records exist.
		</p>
	</section>
{:else if rows.length === 0}
	<p class="muted">No consent records yet.</p>
{:else}
	<p class="muted">{total} record{total === 1 ? '' : 's'} total.</p>
	<table class="table">
		<thead>
			<tr>
				<th>When</th>
				<th>Subject</th>
				<th>Action</th>
				<th>Categories</th>
			</tr>
		</thead>
		<tbody>
			{#each rows as r (r.id)}
				<tr>
					<td>{new Date(r.created_at).toISOString()}</td>
					<td><code>{r.subject_id ?? '(anonymous)'}</code></td>
					<td>{r.action}</td>
					<td><code>{JSON.stringify(r.categories)}</code></td>
				</tr>
			{/each}
		</tbody>
	</table>
	<div class="pager">
		<Button variant="tertiary" size="sm" onclick={prev} disabled={offset === 0}>Prev</Button>
		<span class="muted">{offset + 1}–{Math.min(offset + limit, total)} of {total}</span>
		<Button variant="tertiary" size="sm" onclick={next} disabled={offset + limit >= total}>
			Next
		</Button>
	</div>
{/if}

<section class="integrity">
	<h2>Integrity anchors</h2>
	<p class="muted">
		Each anchor is a SHA-256 digest over an ordered window of <code>consent_records</code> rows.
		Recomputing the hash from the rows in <code>window_start_at..window_end_at</code> must match
		<code>anchor_hash</code> exactly; a mismatch means the log has been tampered with.
	</p>
	{#if anchors.length === 0}
		<p class="muted">No anchors yet. Scheduler will write them hourly once CONSENT-08 lands.</p>
	{:else}
		<table class="table">
			<thead>
				<tr>
					<th>Anchored at</th>
					<th>Records</th>
					<th>Window start</th>
					<th>Window end</th>
					<th>Hash</th>
				</tr>
			</thead>
			<tbody>
				{#each anchors as a (a.id)}
					<tr>
						<td>{new Date(a.anchored_at).toISOString()}</td>
						<td>{a.record_count}</td>
						<td>{a.window_start_at ? new Date(a.window_start_at).toISOString() : '—'}</td>
						<td>{a.window_end_at ? new Date(a.window_end_at).toISOString() : '—'}</td>
						<td><code class="hash">{a.anchor_hash}</code></td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</section>

<style>
	.head {
		margin-block-end: var(--space-5);
	}
	.head h1 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		border: 1px solid var(--surface-border-subtle);
		background-color: var(--surface-bg-canvas);
		margin-block-end: var(--space-4);
	}
	.muted {
		color: var(--surface-fg-muted);
	}
	.empty {
		padding: var(--space-5);
		border: 1px dashed var(--surface-border-subtle);
		border-radius: var(--radius-md);
	}
	.empty h2 {
		margin: 0;
		font-size: var(--fs-lg);
	}
	.empty p {
		margin-block-start: var(--space-2);
		margin-block-end: 0;
		color: var(--surface-fg-muted);
	}
	.table {
		inline-size: 100%;
		border-collapse: collapse;
	}
	.table th,
	.table td {
		padding: var(--space-3);
		border-block-end: 1px solid var(--surface-border-subtle);
		font-size: var(--fs-sm);
		text-align: start;
		vertical-align: top;
	}
	.pager {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-block-start: var(--space-4);
	}
	.integrity {
		margin-block-start: var(--space-8);
	}
	.integrity h2 {
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-semibold);
		margin-block-start: 0;
	}
	.hash {
		font-family: var(--font-mono);
		font-size: var(--fs-2xs);
		word-break: break-all;
	}
</style>
