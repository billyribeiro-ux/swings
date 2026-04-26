<!--
  Phase 2.3 — Consent privacy policy versions. Append-only: every save is a
  brand-new INSERT and old versions are immutable. Creating a new version
  bumps the banner's `policy_version`, which forces re-consent on next page
  load for every visitor.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import ScrollIcon from 'phosphor-svelte/lib/ScrollIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import LockIcon from 'phosphor-svelte/lib/LockIcon';
	import { ApiError } from '$lib/api/client';
	import {
		listPolicies,
		createPolicy,
		type AdminPolicy,
		type PolicyCreateBody
	} from '$lib/api/admin-consent';

	let policies = $state<AdminPolicy[]>([]);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let drawerMode = $state<'create' | 'view' | null>(null);
	let viewing = $state<AdminPolicy | null>(null);

	let formMarkdown = $state('');
	let formLocale = $state('en');
	let formBusy = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			policies = await listPolicies();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load policies';
		} finally {
			loading = false;
		}
	}

	function openCreate() {
		formMarkdown = '';
		formLocale = 'en';
		drawerMode = 'create';
	}

	function openView(p: AdminPolicy) {
		viewing = p;
		drawerMode = 'view';
	}

	function closeDrawer() {
		drawerMode = null;
		viewing = null;
	}

	async function publish() {
		if (!formMarkdown.trim()) {
			error = 'Markdown body is required';
			return;
		}
		formBusy = true;
		error = '';
		try {
			const body: PolicyCreateBody = {
				markdown: formMarkdown,
				locale: formLocale.trim() || 'en'
			};
			const created = await createPolicy(body);
			flash(`Published v${created.version} (${created.locale})`);
			closeDrawer();
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Publish failed';
		} finally {
			formBusy = false;
		}
	}

	function summary(p: AdminPolicy): string {
		const firstLine = p.markdown.split('\n').find((l) => l.trim().length > 0) ?? '';
		const stripped = firstLine.replace(/^#+\s*/, '').trim();
		return stripped.length > 80 ? stripped.slice(0, 80) + '…' : stripped;
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Privacy policy versions · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-consent-policy">
	<header class="page__header">
		<div class="page__title-row">
			<ScrollIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Governance / Consent</span>
				<h1 class="page__title">Privacy policy versions</h1>
				<p class="page__subtitle">
					Each version is an immutable INSERT. Publishing bumps the banner's <code>policy_version</code>
					and forces re-consent UI for every subject on their next page load.
				</p>
			</div>
		</div>
		<div class="page__actions">
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New version</span>
			</button>
			<button class="btn btn--secondary" type="button" onclick={() => void refresh()}>
				<ArrowsClockwiseIcon size={16} weight="bold" />
				<span>Refresh</span>
			</button>
		</div>
	</header>

	<div class="banner">
		<LockIcon size={18} weight="duotone" />
		<div>
			<strong>Append-only.</strong>
			Old versions cannot be edited or deleted — they remain queryable for compliance proofs.
			Publishing a new version is the only way to amend the live policy.
		</div>
	</div>

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

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading policy versions…</span>
		</div>
	{:else if policies.length === 0}
		<div class="empty">
			<ScrollIcon size={48} weight="duotone" />
			<p class="empty__title">No policy versions yet</p>
			<p class="empty__sub">Publish v1 to get started — visitors will be re-prompted to consent.</p>
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New version</span>
			</button>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Version</th>
							<th scope="col">Locale</th>
							<th scope="col">Title</th>
							<th scope="col">Effective at</th>
							<th scope="col">Created</th>
							<th scope="col" class="table__actions-th" aria-label="Actions"></th>
						</tr>
					</thead>
					<tbody>
						{#each policies as p (p.id)}
							<tr>
								<td class="num">v{p.version}</td>
								<td><code>{p.locale}</code></td>
								<td class="desc">{summary(p)}</td>
								<td class="ts">{new Date(p.effective_at).toLocaleString()}</td>
								<td class="ts">{new Date(p.created_at).toLocaleString()}</td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => openView(p)}
										aria-label="View"
									>
										<EyeIcon size={14} weight="bold" />
										<span>View</span>
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{/if}

	{#if drawerMode}
		<div
			class="drawer-backdrop"
			role="button"
			tabindex="-1"
			aria-label="Close"
			onclick={closeDrawer}
			onkeydown={(e) => e.key === 'Escape' && closeDrawer()}
		></div>
		<aside class="drawer" aria-label="Policy editor">
			<header class="drawer__header">
				<h2 class="drawer__title">
					{#if drawerMode === 'create'}New policy version{/if}
					{#if drawerMode === 'view' && viewing}Policy v{viewing.version} · {viewing.locale}{/if}
				</h2>
				<button class="btn btn--secondary btn--small" type="button" onclick={closeDrawer}>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
				</button>
			</header>
			{#if drawerMode === 'create'}
				<div class="form">
					<div class="field">
						<label class="field__label" for="pol-locale">Locale</label>
						<input
							id="pol-locale"
							name="pol-locale"
							class="field__input"
							placeholder="en"
							bind:value={formLocale}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="pol-md">Markdown body</label>
						<textarea
							id="pol-md"
							name="pol-md"
							class="field__input field__input--mono field__input--xtall"
							rows={18}
							bind:value={formMarkdown}
						></textarea>
					</div>
					<p class="hint">
						Whitespace is preserved verbatim — paste from your legal review tool without
						re-formatting.
					</p>
					<div class="form__actions">
						<button class="btn btn--primary" type="button" disabled={formBusy} onclick={publish}>
							<CheckCircleIcon size={16} weight="bold" />
							<span>{formBusy ? 'Publishing…' : 'Publish version'}</span>
						</button>
						<button class="btn btn--secondary" type="button" onclick={closeDrawer}>
							<XIcon size={16} weight="bold" />
							<span>Cancel</span>
						</button>
					</div>
				</div>
			{:else if drawerMode === 'view' && viewing}
				<dl class="meta">
					<dt>Version</dt><dd>v{viewing.version}</dd>
					<dt>Locale</dt><dd><code>{viewing.locale}</code></dd>
					<dt>Effective at</dt><dd>{new Date(viewing.effective_at).toLocaleString()}</dd>
					<dt>Created</dt><dd>{new Date(viewing.created_at).toLocaleString()}</dd>
				</dl>
				<details open>
					<summary>Markdown body</summary>
					<pre class="markdown">{viewing.markdown}</pre>
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
	.page__subtitle code { font-size: 0.85em; padding: 0.1em 0.35em; border-radius: 0.25rem; background: rgba(255, 255, 255, 0.06); }

	.banner { display: flex; gap: 0.75rem; align-items: flex-start; padding: 0.85rem 1rem; margin-bottom: 1.25rem; background: rgba(245, 158, 11, 0.08); border: 1px solid rgba(245, 158, 11, 0.25); border-radius: var(--radius-lg); color: #fcd34d; font-size: 0.875rem; line-height: 1.5; }
	.banner :global(svg) { color: #fcd34d; flex-shrink: 0; margin-top: 0.1rem; }
	.banner strong { color: #fde68a; }

	.toast, .error { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; border-radius: var(--radius-lg); font-size: 0.875rem; margin-bottom: 1rem; }
	.toast { background: rgba(15, 164, 175, 0.12); border: 1px solid rgba(15, 164, 175, 0.25); color: #5eead4; }
	.error { background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5; }

	.state { display: flex; align-items: center; justify-content: center; gap: 0.75rem; padding: 4rem 0; color: var(--color-grey-400); font-size: 0.875rem; }
	.state__spinner { width: 1.25rem; height: 1.25rem; border: 2px solid rgba(255, 255, 255, 0.1); border-top-color: var(--color-teal); border-radius: 50%; animation: spin 0.7s linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }

	.empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 3rem 1rem; background: var(--color-navy-mid); border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: var(--radius-xl); color: var(--color-grey-500); text-align: center; }
	.empty :global(svg) { color: var(--color-grey-500); }
	.empty__title { margin: 0.5rem 0 0; font-size: 1rem; font-weight: 600; color: var(--color-white); }
	.empty__sub { margin: 0 0 0.75rem; font-size: 0.875rem; color: var(--color-grey-400); }

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
	.num { font-variant-numeric: tabular-nums; color: var(--color-grey-300); font-weight: 600; }
	.desc { color: var(--color-grey-300); max-width: 40ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

	.btn { display: inline-flex; align-items: center; gap: 0.5rem; min-height: 2.5rem; padding: 0 0.875rem; border-radius: var(--radius-lg); font-size: 0.8125rem; font-weight: 600; border: 1px solid transparent; background: transparent; color: var(--color-grey-300); cursor: pointer; transition: background-color 150ms, border-color 150ms, color 150ms, box-shadow 150ms, transform 150ms; }
	.btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn--primary { background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94)); color: var(--color-white); box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45); }
	.btn--primary:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55); }
	.btn--secondary { background: rgba(255, 255, 255, 0.05); border-color: rgba(255, 255, 255, 0.1); color: var(--color-grey-200); }
	.btn--secondary:hover:not(:disabled) { background: rgba(255, 255, 255, 0.1); border-color: rgba(255, 255, 255, 0.18); color: var(--color-white); }
	.btn--small { min-height: 2rem; padding: 0 0.65rem; font-size: 0.75rem; }

	.drawer-backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); z-index: 60; }
	.drawer { position: fixed; top: 0; right: 0; bottom: 0; width: min(720px, 92vw); background: var(--color-navy); border-left: 1px solid rgba(255, 255, 255, 0.08); padding: 1.5rem; overflow-y: auto; z-index: 70; box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3); }
	.drawer__header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
	.drawer__title { font-family: var(--font-heading); font-size: 1rem; font-weight: 600; color: var(--color-white); margin: 0; letter-spacing: -0.01em; }
	.form { display: flex; flex-direction: column; gap: 0.85rem; }
	.form__actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.5rem; }
	.field { display: flex; flex-direction: column; gap: 0.4rem; }
	.field__label { font-size: 0.75rem; color: var(--color-grey-300); font-weight: 500; }
	.field__input { min-height: 2.5rem; padding: 0.65rem 0.875rem; background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); color: var(--color-white); border-radius: var(--radius-lg); font-size: 0.875rem; width: 100%; font-family: inherit; color-scheme: dark; transition: border-color 150ms, box-shadow 150ms; }
	.field__input--mono { font-family: var(--font-mono); font-size: 0.78rem; }
	.field__input--xtall { min-height: 22rem; }
	.field__input::placeholder { color: var(--color-grey-500); }
	.field__input:focus { outline: none; border-color: var(--color-teal); box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15); }
	.hint { margin: 0; font-size: 0.75rem; color: var(--color-grey-500); line-height: 1.45; }
	.meta { display: grid; grid-template-columns: 8rem 1fr; gap: 0.5rem 0.85rem; font-size: 0.875rem; color: var(--color-grey-200); margin-bottom: 1rem; }
	.meta dt { color: var(--color-grey-500); font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.meta dd { margin: 0; word-break: break-all; }
	.markdown { background: rgba(0, 0, 0, 0.3); padding: 0.85rem; border-radius: var(--radius-lg); font-size: 0.78rem; color: var(--color-grey-200); max-height: 60vh; overflow: auto; white-space: pre-wrap; word-break: break-word; font-family: var(--font-mono); }
</style>
