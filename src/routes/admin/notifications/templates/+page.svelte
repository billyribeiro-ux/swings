<!--
  Phase 2.1 — Notification templates list + create + edit drawer.
  Wraps `/api/admin/notifications/templates` (list / get / create / update /
  preview / test-send). Updates are version-bumped server-side: every save is
  a NEW row and the table groups by (key, channel, locale) showing the
  highest version per group.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import EnvelopeIcon from 'phosphor-svelte/lib/EnvelopeIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import PaperPlaneTiltIcon from 'phosphor-svelte/lib/PaperPlaneTiltIcon';
	import PencilIcon from 'phosphor-svelte/lib/PencilIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import { ApiError } from '$lib/api/client';
	import {
		templates,
		type CreateTemplateBody,
		type PaginatedTemplatesResponse,
		type RenderedTemplate,
		type Template,
		type TemplateListQuery,
		type UpdateTemplateBody
	} from '$lib/api/admin-notifications';

	type DrawerMode = 'create' | 'edit' | 'preview' | 'test-send';

	let envelope = $state<PaginatedTemplatesResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let filters = $state<TemplateListQuery>({
		key: '',
		channel: '',
		locale: '',
		active_only: false,
		page: 1,
		per_page: 25
	});

	let drawerMode = $state<DrawerMode | null>(null);
	let editing = $state<Template | null>(null);

	// Editor form state — reset on every drawer open.
	let formKey = $state('');
	let formChannel = $state('email');
	let formLocale = $state('en');
	let formSubject = $state('');
	let formBody = $state('');
	let formActive = $state(true);
	let formBusy = $state(false);

	// Preview / test-send state.
	let previewContext = $state('{\n  "name": "Pat"\n}');
	let preview = $state<RenderedTemplate | null>(null);
	let previewBusy = $state(false);
	let testTo = $state('');
	let testBusy = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	function buildQuery(): TemplateListQuery {
		const q: TemplateListQuery = {
			page: filters.page ?? 1,
			per_page: filters.per_page ?? 25
		};
		if (filters.key?.trim()) q.key = filters.key.trim();
		if (filters.channel?.trim()) q.channel = filters.channel.trim();
		if (filters.locale?.trim()) q.locale = filters.locale.trim();
		if (filters.active_only) q.active_only = true;
		return q;
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await templates.list(buildQuery());
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load templates';
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
		filters = {
			key: '',
			channel: '',
			locale: '',
			active_only: false,
			page: 1,
			per_page: 25
		};
		void refresh();
	}

	function openCreate() {
		editing = null;
		formKey = '';
		formChannel = 'email';
		formLocale = 'en';
		formSubject = '';
		formBody = '';
		formActive = true;
		drawerMode = 'create';
	}

	function openEdit(row: Template) {
		editing = row;
		formKey = row.key;
		formChannel = row.channel;
		formLocale = row.locale;
		formSubject = row.subject ?? '';
		formBody = row.body_source;
		formActive = row.is_active;
		drawerMode = 'edit';
	}

	function openPreview(row: Template) {
		editing = row;
		preview = null;
		previewContext = '{\n  "name": "Pat"\n}';
		drawerMode = 'preview';
	}

	function openTestSend(row: Template) {
		editing = row;
		testTo = '';
		previewContext = '{\n  "name": "Pat"\n}';
		drawerMode = 'test-send';
	}

	function closeDrawer() {
		drawerMode = null;
		editing = null;
		preview = null;
	}

	async function saveTemplate() {
		if (!formBody.trim()) {
			error = 'Body is required';
			return;
		}
		formBusy = true;
		error = '';
		try {
			if (drawerMode === 'create') {
				if (!formKey.trim()) {
					error = 'Key is required';
					formBusy = false;
					return;
				}
				const body: CreateTemplateBody = {
					key: formKey.trim(),
					channel: formChannel,
					locale: formLocale,
					subject: formSubject.trim() || null,
					body_source: formBody,
					is_active: formActive
				};
				await templates.create(body);
				flash('Template created');
			} else if (drawerMode === 'edit' && editing) {
				const body: UpdateTemplateBody = {
					subject: formSubject.trim() || null,
					body_source: formBody,
					is_active: formActive
				};
				await templates.update(editing.id, body);
				flash(`v${editing.version + 1} saved`);
			}
			closeDrawer();
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Save failed';
		} finally {
			formBusy = false;
		}
	}

	function parseContext(): unknown | null {
		try {
			return JSON.parse(previewContext);
		} catch {
			error = 'Context must be valid JSON';
			return null;
		}
	}

	async function runPreview() {
		if (!editing) return;
		const ctx = parseContext();
		if (ctx === null) return;
		previewBusy = true;
		error = '';
		try {
			preview = await templates.preview(editing.id, { context: ctx });
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Preview failed';
		} finally {
			previewBusy = false;
		}
	}

	async function runTestSend() {
		if (!editing) return;
		if (!testTo.trim()) {
			error = '`to` is required';
			return;
		}
		const ctx = parseContext();
		if (ctx === null) return;
		testBusy = true;
		error = '';
		try {
			const r = await templates.testSend(editing.id, { to: testTo.trim(), context: ctx });
			flash(`Sent · provider id ${r.provider_id.slice(0, 12)}…`);
			closeDrawer();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Test send failed';
		} finally {
			testBusy = false;
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

	const summaryRange = $derived.by(() => {
		if (!envelope) return '';
		const start = ((envelope.page ?? 1) - 1) * (envelope.per_page ?? 25) + 1;
		const end = start + envelope.data.length - 1;
		return `${start}–${end} of ${envelope.total}`;
	});

	function statusPillClass(active: boolean): string {
		return active ? 'pill pill--success' : 'pill pill--neutral';
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Email templates · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-notifications-templates">
	<header class="page__header">
		<div class="page__title-row">
			<EnvelopeIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Operations / Notifications</span>
				<h1 class="page__title">Email templates</h1>
				<p class="page__subtitle">
					Versioned transactional and marketing templates. Saving an existing template appends a new
					version — older versions stay queryable in the audit log.
				</p>
			</div>
		</div>
		<div class="page__actions">
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New template</span>
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
			<div class="field field--wide">
				<label class="field__label" for="tpl-key">Search by key</label>
				<div class="search-input">
					<MagnifyingGlassIcon size={16} />
					<input
						id="tpl-key"
						name="tpl-key"
						class="field__input field__input--search"
						placeholder="user.welcome, order.confirmed, …"
						bind:value={filters.key}
					/>
				</div>
			</div>
			<div class="field">
				<label class="field__label" for="tpl-channel">Channel</label>
				<select
					id="tpl-channel"
					name="tpl-channel"
					class="field__input"
					bind:value={filters.channel}
				>
					<option value="">Any</option>
					<option value="email">Email</option>
					<option value="sms">SMS</option>
					<option value="push">Push</option>
					<option value="in_app">In-app</option>
					<option value="slack">Slack</option>
					<option value="discord">Discord</option>
					<option value="webhook">Webhook</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="tpl-locale">Locale</label>
				<input
					id="tpl-locale"
					name="tpl-locale"
					class="field__input"
					placeholder="en, en-US, fr, …"
					bind:value={filters.locale}
				/>
			</div>
			<div class="field field--inline">
				<label class="check-row" for="tpl-active">
					<input
						id="tpl-active"
						name="tpl-active"
						type="checkbox"
						bind:checked={filters.active_only}
					/>
					<span>Active only</span>
				</label>
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
			<span>Loading templates…</span>
		</div>
	{:else if !envelope || envelope.data.length === 0}
		<div class="empty">
			<EnvelopeIcon size={48} weight="duotone" />
			<p class="empty__title">No templates match</p>
			<p class="empty__sub">Create one or widen the filters.</p>
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New template</span>
			</button>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Key</th>
							<th scope="col">Channel</th>
							<th scope="col">Locale</th>
							<th scope="col">Version</th>
							<th scope="col">Updated</th>
							<th scope="col">Status</th>
							<th scope="col" class="table__actions-th" aria-label="Actions"></th>
						</tr>
					</thead>
					<tbody>
						{#each envelope.data as row (row.id)}
							<tr>
								<td><code class="key">{row.key}</code></td>
								<td><span class="pill pill--neutral">{row.channel}</span></td>
								<td><code>{row.locale}</code></td>
								<td class="num">v{row.version}</td>
								<td class="ts">{new Date(row.updated_at).toLocaleString()}</td>
								<td>
									<span class={statusPillClass(row.is_active)}>
										{row.is_active ? 'Active' : 'Draft'}
									</span>
								</td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => openEdit(row)}
										aria-label="Edit"
									>
										<PencilIcon size={14} weight="bold" />
										<span>Edit</span>
									</button>
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => openPreview(row)}
										aria-label="Preview"
									>
										<EyeIcon size={14} weight="bold" />
										<span>Preview</span>
									</button>
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => openTestSend(row)}
										aria-label="Test send"
									>
										<PaperPlaneTiltIcon size={14} weight="bold" />
										<span>Test</span>
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

	{#if drawerMode}
		<div
			class="drawer-backdrop"
			role="button"
			tabindex="-1"
			aria-label="Close"
			onclick={closeDrawer}
			onkeydown={(e) => e.key === 'Escape' && closeDrawer()}
		></div>
		<aside class="drawer" aria-label="Template editor">
			<header class="drawer__header">
				<h2 class="drawer__title">
					{#if drawerMode === 'create'}New template{/if}
					{#if drawerMode === 'edit' && editing}Edit · {editing.key}{/if}
					{#if drawerMode === 'preview' && editing}Preview · {editing.key}{/if}
					{#if drawerMode === 'test-send' && editing}Test send · {editing.key}{/if}
				</h2>
				<button class="btn btn--secondary btn--small" type="button" onclick={closeDrawer}>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
				</button>
			</header>

			{#if drawerMode === 'create' || drawerMode === 'edit'}
				<div class="form">
					<div class="grid-2">
						<div class="field">
							<label class="field__label" for="form-key">Key</label>
							<input
								id="form-key"
								name="form-key"
								class="field__input"
								placeholder="user.welcome"
								bind:value={formKey}
								disabled={drawerMode === 'edit'}
								required
							/>
						</div>
						<div class="field">
							<label class="field__label" for="form-channel">Channel</label>
							<select
								id="form-channel"
								name="form-channel"
								class="field__input"
								bind:value={formChannel}
								disabled={drawerMode === 'edit'}
							>
								<option value="email">Email</option>
								<option value="sms">SMS</option>
								<option value="push">Push</option>
								<option value="in_app">In-app</option>
								<option value="slack">Slack</option>
								<option value="discord">Discord</option>
								<option value="webhook">Webhook</option>
							</select>
						</div>
					</div>
					<div class="grid-2">
						<div class="field">
							<label class="field__label" for="form-locale">Locale</label>
							<input
								id="form-locale"
								name="form-locale"
								class="field__input"
								placeholder="en"
								bind:value={formLocale}
								disabled={drawerMode === 'edit'}
							/>
						</div>
						<div class="field field--inline">
							<label class="check-row" for="form-active">
								<input
									id="form-active"
									name="form-active"
									type="checkbox"
									bind:checked={formActive}
								/>
								<span>Active</span>
							</label>
						</div>
					</div>
					<div class="field">
						<label class="field__label" for="form-subject">Subject</label>
						<input
							id="form-subject"
							name="form-subject"
							class="field__input"
							placeholder="Welcome, {'{{ name }}'}"
							bind:value={formSubject}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="form-body">Body (HTML / MJML)</label>
						<textarea
							id="form-body"
							name="form-body"
							class="field__input field__input--mono field__input--tall"
							rows={14}
							bind:value={formBody}
						></textarea>
					</div>
					<div class="form__actions">
						<button
							class="btn btn--primary"
							type="button"
							disabled={formBusy}
							onclick={saveTemplate}
						>
							<CheckCircleIcon size={16} weight="bold" />
							<span>{formBusy ? 'Saving…' : drawerMode === 'create' ? 'Create' : 'Save new version'}</span>
						</button>
						<button class="btn btn--secondary" type="button" onclick={closeDrawer}>
							<XIcon size={16} weight="bold" />
							<span>Cancel</span>
						</button>
					</div>
				</div>
			{:else if drawerMode === 'preview'}
				<div class="form">
					<div class="field">
						<label class="field__label" for="prev-ctx">Context (JSON)</label>
						<textarea
							id="prev-ctx"
							name="prev-ctx"
							class="field__input field__input--mono"
							rows={6}
							bind:value={previewContext}
						></textarea>
					</div>
					<div class="form__actions">
						<button class="btn btn--primary" type="button" disabled={previewBusy} onclick={runPreview}>
							<EyeIcon size={16} weight="bold" />
							<span>{previewBusy ? 'Rendering…' : 'Render preview'}</span>
						</button>
					</div>
					{#if preview}
						<dl class="meta">
							<dt>Subject</dt>
							<dd>{preview.subject ?? '—'}</dd>
						</dl>
						<details open>
							<summary>Rendered body</summary>
							<pre class="json">{preview.body}</pre>
						</details>
					{/if}
				</div>
			{:else if drawerMode === 'test-send'}
				<div class="form">
					<div class="field">
						<label class="field__label" for="test-to">Recipient address</label>
						<input
							id="test-to"
							name="test-to"
							class="field__input"
							type="email"
							placeholder="ops@example.com"
							bind:value={testTo}
							required
						/>
					</div>
					<div class="field">
						<label class="field__label" for="test-ctx">Context (JSON)</label>
						<textarea
							id="test-ctx"
							name="test-ctx"
							class="field__input field__input--mono"
							rows={6}
							bind:value={previewContext}
						></textarea>
					</div>
					<p class="hint">
						This dispatches via the live channel provider. Use a controlled mailbox.
					</p>
					<div class="form__actions">
						<button class="btn btn--primary" type="button" disabled={testBusy} onclick={runTestSend}>
							<PaperPlaneTiltIcon size={16} weight="bold" />
							<span>{testBusy ? 'Sending…' : 'Send test'}</span>
						</button>
						<button class="btn btn--secondary" type="button" onclick={closeDrawer}>
							<XIcon size={16} weight="bold" />
							<span>Cancel</span>
						</button>
					</div>
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

	.toast, .error { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; border-radius: var(--radius-lg); font-size: 0.875rem; margin-bottom: 1rem; }
	.toast { background: rgba(15, 164, 175, 0.12); border: 1px solid rgba(15, 164, 175, 0.25); color: #5eead4; }
	.error { background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5; }

	.filters { background: var(--color-navy-mid); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-xl); padding: 1.25rem; margin-bottom: 1.25rem; box-shadow: 0 1px 0 rgba(255, 255, 255, 0.03) inset, 0 12px 32px rgba(0, 0, 0, 0.18); }
	.filters__head { margin-bottom: 0.85rem; }
	.filters__eyebrow { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.6875rem; font-weight: 700; color: var(--color-grey-500); text-transform: uppercase; letter-spacing: 0.06em; }
	.filters__grid { display: grid; grid-template-columns: 1fr; gap: 0.85rem; }
	.filters__actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 1rem; align-items: center; }

	.field { display: flex; flex-direction: column; gap: 0.4rem; }
	.field--wide { grid-column: span 1; }
	.field--inline { justify-content: flex-end; }
	.field__label { font-size: 0.75rem; color: var(--color-grey-300); font-weight: 500; }
	.field__input { min-height: 2.5rem; padding: 0.65rem 0.875rem; background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); color: var(--color-white); border-radius: var(--radius-lg); font-size: 0.875rem; width: 100%; font-family: inherit; color-scheme: dark; transition: border-color 150ms, box-shadow 150ms; }
	.field__input::placeholder { color: var(--color-grey-500); }
	.field__input--mono { font-family: var(--font-mono); font-size: 0.78rem; }
	.field__input--tall { min-height: 14rem; }
	.field__input:focus { outline: none; border-color: var(--color-teal); box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15); }
	.field__input:disabled { opacity: 0.55; cursor: not-allowed; }
	.search-input { position: relative; }
	.search-input :global(svg) { position: absolute; left: 0.75rem; top: 50%; transform: translateY(-50%); color: var(--color-grey-400); pointer-events: none; }
	.field__input--search { padding-left: 2.25rem; }
	.check-row { display: inline-flex; gap: 0.5rem; align-items: center; font-size: 0.875rem; color: var(--color-grey-200); cursor: pointer; }
	.check-row input { accent-color: var(--color-teal); }

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
	.num { font-variant-numeric: tabular-nums; text-align: right; color: var(--color-grey-300); }
	.key { color: var(--color-teal-light); }

	.pill { display: inline-flex; align-items: center; padding: 0.15rem 0.5rem; border-radius: var(--radius-full); font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
	.pill--success { background: rgba(15, 164, 175, 0.12); color: #5eead4; }
	.pill--neutral { background: rgba(255, 255, 255, 0.06); color: var(--color-grey-300); }

	.pager { display: flex; gap: 0.75rem; justify-content: center; align-items: center; margin-top: 1.25rem; flex-wrap: wrap; }
	.pager__info { font-size: 0.75rem; font-weight: 500; color: var(--color-grey-400); font-variant-numeric: tabular-nums; }

	.drawer-backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); z-index: 60; }
	.drawer { position: fixed; top: 0; right: 0; bottom: 0; width: min(640px, 92vw); background: var(--color-navy); border-left: 1px solid rgba(255, 255, 255, 0.08); padding: 1.5rem; overflow-y: auto; z-index: 70; box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3); }
	.drawer__header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; gap: 0.75rem; }
	.drawer__title { font-family: var(--font-heading); font-size: 1rem; font-weight: 600; color: var(--color-white); margin: 0; letter-spacing: -0.01em; min-width: 0; word-break: break-all; }
	.form { display: flex; flex-direction: column; gap: 0.85rem; }
	.form__actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.5rem; }
	.grid-2 { display: grid; grid-template-columns: 1fr; gap: 0.85rem; }
	.hint { margin: 0; font-size: 0.75rem; color: var(--color-grey-500); line-height: 1.45; }
	.meta { display: grid; grid-template-columns: 6rem 1fr; gap: 0.5rem 0.85rem; font-size: 0.875rem; color: var(--color-grey-200); margin: 0.5rem 0; }
	.meta dt { color: var(--color-grey-500); font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.meta dd { margin: 0; word-break: break-all; }
	.json { background: rgba(0, 0, 0, 0.3); padding: 0.85rem; border-radius: var(--radius-lg); font-size: 0.75rem; color: var(--color-grey-200); max-height: 50vh; overflow: auto; white-space: pre-wrap; word-break: break-all; }

	@media (min-width: 480px) {
		.filters__grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
		.grid-2 { grid-template-columns: 1fr 1fr; }
	}
	@media (min-width: 768px) {
		.filters { padding: 1.75rem; border-radius: var(--radius-2xl); }
		.filters__grid { grid-template-columns: repeat(4, minmax(0, 1fr)); }
		.field--wide { grid-column: span 2; }
	}
</style>
