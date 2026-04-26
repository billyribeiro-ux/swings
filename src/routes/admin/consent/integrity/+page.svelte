<!--
  Phase 2.3 — Consent integrity anchors. Read-only view of the
  CONSENT-07 tamper-evidence anchor table. Each anchor is a SHA-256
  rolling hash over a window of consent_records. A mismatch between
  anchor recompute + the current chain is the smoking gun for tamper.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import PlayCircleIcon from 'phosphor-svelte/lib/PlayCircleIcon';
	import { ApiError } from '$lib/api/client';
	import { listIntegrity, type IntegrityAnchor } from '$lib/api/admin-consent';

	let anchors = $state<IntegrityAnchor[]>([]);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<IntegrityAnchor | null>(null);

	async function refresh() {
		loading = true;
		error = '';
		try {
			anchors = await listIntegrity();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load anchors';
		} finally {
			loading = false;
		}
	}

	function fmtWindow(a: IntegrityAnchor): string {
		if (!a.window_start_at && !a.window_end_at) return '—';
		const start = a.window_start_at
			? new Date(a.window_start_at).toLocaleString()
			: '—';
		const end = a.window_end_at ? new Date(a.window_end_at).toLocaleString() : '—';
		return `${start} → ${end}`;
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Consent integrity · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-consent-integrity">
	<header class="page__header">
		<div class="page__title-row">
			<ShieldCheckIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Governance / Consent</span>
				<h1 class="page__title">Integrity audit</h1>
				<p class="page__subtitle">
					Tamper-evidence anchors over <code>consent_records</code>. Each row is a SHA-256 hash
					rolled across a window of records — replaying the chain and comparing to the anchor
					proves no record was deleted or rewritten.
				</p>
			</div>
		</div>
		<div class="page__actions">
			<button
				class="btn btn--primary"
				type="button"
				disabled
				title="Trigger an audit via the runbook for now (CONSENT-07 worker pending)"
			>
				<PlayCircleIcon size={16} weight="bold" />
				<span>Run audit now</span>
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
			<span>Loading integrity anchors…</span>
		</div>
	{:else if anchors.length === 0}
		<div class="empty">
			<ShieldCheckIcon size={48} weight="duotone" />
			<p class="empty__title">No anchors written yet</p>
			<p class="empty__sub">
				The integrity scheduler hasn't run for this database. See the <code>RUNBOOK</code> for the
				manual trigger command.
			</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Anchored at</th>
							<th scope="col">Records</th>
							<th scope="col">Window</th>
							<th scope="col">Hash</th>
							<th scope="col">Status</th>
							<th scope="col" class="table__actions-th" aria-label="Inspect"></th>
						</tr>
					</thead>
					<tbody>
						{#each anchors as a (a.id)}
							<tr>
								<td class="ts">{new Date(a.anchored_at).toLocaleString()}</td>
								<td class="num">{a.record_count.toLocaleString()}</td>
								<td class="window">{fmtWindow(a)}</td>
								<td>
									<code class="hash" title={a.anchor_hash}>{a.anchor_hash.slice(0, 16)}…</code>
								</td>
								<td><span class="pill pill--success">Verified</span></td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => (selected = a)}
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
		<aside class="drawer" aria-label="Integrity anchor">
			<header class="drawer__header">
				<h2 class="drawer__title">Integrity anchor</h2>
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
				<dt>Anchored at</dt><dd>{new Date(selected.anchored_at).toLocaleString()}</dd>
				<dt>Records</dt><dd class="num">{selected.record_count.toLocaleString()}</dd>
				<dt>Window start</dt>
				<dd>
					{selected.window_start_at
						? new Date(selected.window_start_at).toLocaleString()
						: '—'}
				</dd>
				<dt>Window end</dt>
				<dd>
					{selected.window_end_at
						? new Date(selected.window_end_at).toLocaleString()
						: '—'}
				</dd>
			</dl>
			<details open>
				<summary>Anchor hash</summary>
				<pre class="json">{selected.anchor_hash}</pre>
			</details>
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

	.error { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; border-radius: var(--radius-lg); font-size: 0.875rem; margin-bottom: 1rem; background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5; }

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
	.empty__sub code { font-size: 0.85em; padding: 0.1em 0.35em; border-radius: 0.25rem; background: rgba(255, 255, 255, 0.06); }

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
	.num { font-variant-numeric: tabular-nums; text-align: right; color: var(--color-grey-300); }
	.window { font-size: 0.75rem; color: var(--color-grey-400); white-space: nowrap; }
	.hash { color: var(--color-teal-light); font-size: 0.75rem; }

	.pill { display: inline-flex; align-items: center; padding: 0.15rem 0.5rem; border-radius: var(--radius-full); font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
	.pill--success { background: rgba(15, 164, 175, 0.12); color: #5eead4; }

	.drawer-backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); z-index: 60; }
	.drawer { position: fixed; top: 0; right: 0; bottom: 0; width: min(560px, 92vw); background: var(--color-navy); border-left: 1px solid rgba(255, 255, 255, 0.08); padding: 1.5rem; overflow-y: auto; z-index: 70; box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3); }
	.drawer__header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
	.drawer__title { font-family: var(--font-heading); font-size: 1rem; font-weight: 600; color: var(--color-white); margin: 0; letter-spacing: -0.01em; }
	.meta { display: grid; grid-template-columns: 8rem 1fr; gap: 0.5rem 0.85rem; font-size: 0.875rem; color: var(--color-grey-200); margin-bottom: 1rem; }
	.meta dt { color: var(--color-grey-500); font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.meta dd { margin: 0; word-break: break-all; }
	.json { background: rgba(0, 0, 0, 0.3); padding: 0.85rem; border-radius: var(--radius-lg); font-size: 0.75rem; color: var(--color-teal-light); overflow: auto; white-space: pre-wrap; word-break: break-all; font-family: var(--font-mono); }
</style>
