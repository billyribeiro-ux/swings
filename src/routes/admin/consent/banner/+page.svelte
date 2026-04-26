<!--
  Phase 2.3 — Consent banner configurations. List of (region × locale) cards
  with a side drawer for create / edit. Each save calls
  POST/PUT /api/admin/consent/banners and bumps the server-side `version`.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import MegaphoneIcon from 'phosphor-svelte/lib/MegaphoneIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import PencilIcon from 'phosphor-svelte/lib/PencilIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import { ApiError } from '$lib/api/client';
	import {
		listBanners,
		createBanner,
		updateBanner,
		type AdminBannerConfig,
		type BannerUpsertBody
	} from '$lib/api/admin-consent';

	type DrawerMode = 'create' | 'edit' | null;

	let banners = $state<AdminBannerConfig[]>([]);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let mode = $state<DrawerMode>(null);
	let editingId = $state<string | null>(null);

	let region = $state('default');
	let locale = $state('en');
	let layout = $state('bar');
	let position = $state('bottom');
	let isActive = $state(true);
	let copyTitle = $state('');
	let copyBody = $state('');
	let copyAcceptAll = $state('Accept all');
	let copyRejectAll = $state('Reject all');
	let copyCustomize = $state('Customize');
	let copySavePreferences = $state('Save preferences');
	let copyPrivacyHref = $state('/privacy');
	let copyPrivacyLabel = $state('Privacy policy');
	let formBusy = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			banners = await listBanners();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load banners';
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		region = 'default';
		locale = 'en';
		layout = 'bar';
		position = 'bottom';
		isActive = true;
		copyTitle = '';
		copyBody = '';
		copyAcceptAll = 'Accept all';
		copyRejectAll = 'Reject all';
		copyCustomize = 'Customize';
		copySavePreferences = 'Save preferences';
		copyPrivacyHref = '/privacy';
		copyPrivacyLabel = 'Privacy policy';
	}

	function openCreate() {
		resetForm();
		editingId = null;
		mode = 'create';
	}

	function openEdit(b: AdminBannerConfig) {
		editingId = b.id;
		region = b.region;
		locale = b.locale;
		layout = b.layout;
		position = b.position;
		isActive = b.is_active;
		const c = b.copy_json as Record<string, unknown>;
		copyTitle = String(c?.title ?? '');
		copyBody = String(c?.body ?? '');
		copyAcceptAll = String(c?.accept_all ?? 'Accept all');
		copyRejectAll = String(c?.reject_all ?? 'Reject all');
		copyCustomize = String(c?.customize ?? 'Customize');
		copySavePreferences = String(c?.save_preferences ?? 'Save preferences');
		const privacy = (c?.privacy ?? {}) as Record<string, unknown>;
		copyPrivacyHref = String(privacy?.href ?? '/privacy');
		copyPrivacyLabel = String(privacy?.label ?? 'Privacy policy');
		mode = 'edit';
	}

	function closeDrawer() {
		mode = null;
		editingId = null;
	}

	async function save() {
		formBusy = true;
		error = '';
		const body: BannerUpsertBody = {
			region,
			locale,
			layout,
			position,
			is_active: isActive,
			copy_json: {
				title: copyTitle,
				body: copyBody,
				accept_all: copyAcceptAll,
				reject_all: copyRejectAll,
				customize: copyCustomize,
				save_preferences: copySavePreferences,
				privacy: { href: copyPrivacyHref, label: copyPrivacyLabel }
			},
			theme_json: {}
		};
		try {
			if (mode === 'create') {
				await createBanner(body);
				flash('Banner created');
			} else if (editingId) {
				await updateBanner(editingId, body);
				flash('Banner updated');
			}
			closeDrawer();
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Save failed';
		} finally {
			formBusy = false;
		}
	}

	async function setLive(b: AdminBannerConfig) {
		formBusy = true;
		error = '';
		try {
			const body: BannerUpsertBody = {
				region: b.region,
				locale: b.locale,
				layout: b.layout,
				position: b.position,
				copy_json: b.copy_json,
				theme_json: b.theme_json,
				is_active: true
			};
			await updateBanner(b.id, body);
			flash(`${b.region}/${b.locale} set live`);
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to set live';
		} finally {
			formBusy = false;
		}
	}

	function bannerTitle(b: AdminBannerConfig): string {
		const c = b.copy_json as Record<string, unknown>;
		return String(c?.title ?? `${b.region}/${b.locale}`);
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Consent banners · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-consent-banner">
	<header class="page__header">
		<div class="page__title-row">
			<MegaphoneIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Governance / Consent</span>
				<h1 class="page__title">Banners</h1>
				<p class="page__subtitle">
					One config per (region, locale). The active banner shapes copy + button labels for every
					visitor; saving an existing config bumps the server-side version.
				</p>
			</div>
		</div>
		<div class="page__actions">
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New banner</span>
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

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading banners…</span>
		</div>
	{:else if banners.length === 0}
		<div class="empty">
			<MegaphoneIcon size={48} weight="duotone" />
			<p class="empty__title">No banner configs yet</p>
			<p class="empty__sub">Create one for the default region to get started.</p>
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New banner</span>
			</button>
		</div>
	{:else}
		<div class="grid">
			{#each banners as b (b.id)}
				<article class="card">
					<header class="card__head">
						<span class="card__eyebrow">{b.region} · {b.locale}</span>
						<span class={b.is_active ? 'pill pill--success' : 'pill pill--neutral'}>
							{b.is_active ? 'Live' : 'Draft'}
						</span>
					</header>
					<h2 class="card__title">{bannerTitle(b)}</h2>
					<dl class="card__meta">
						<dt>Layout</dt><dd><code>{b.layout}</code></dd>
						<dt>Position</dt><dd><code>{b.position}</code></dd>
						<dt>Version</dt><dd>v{b.version}</dd>
						<dt>Updated</dt><dd>{new Date(b.updated_at).toLocaleString()}</dd>
					</dl>
					<div class="card__actions">
						<button
							class="btn btn--secondary btn--small"
							type="button"
							onclick={() => openEdit(b)}
						>
							<PencilIcon size={14} weight="bold" />
							<span>Edit</span>
						</button>
						{#if !b.is_active}
							<button
								class="btn btn--primary btn--small"
								type="button"
								onclick={() => setLive(b)}
								disabled={formBusy}
							>
								<CheckCircleIcon size={14} weight="bold" />
								<span>Set live</span>
							</button>
						{/if}
					</div>
				</article>
			{/each}
		</div>
	{/if}

	{#if mode}
		<div
			class="drawer-backdrop"
			role="button"
			tabindex="-1"
			aria-label="Close"
			onclick={closeDrawer}
			onkeydown={(e) => e.key === 'Escape' && closeDrawer()}
		></div>
		<aside class="drawer" aria-label="Banner editor">
			<header class="drawer__header">
				<h2 class="drawer__title">
					{mode === 'create' ? 'New banner' : 'Edit banner'}
				</h2>
				<button
					class="btn btn--secondary btn--small"
					type="button"
					onclick={closeDrawer}
				>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
				</button>
			</header>
			<div class="form">
				<div class="grid-2">
					<div class="field">
						<label class="field__label" for="bn-region">Region</label>
						<input
							id="bn-region"
							name="bn-region"
							class="field__input"
							placeholder="default, eu, …"
							bind:value={region}
							disabled={mode === 'edit'}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="bn-locale">Locale</label>
						<input
							id="bn-locale"
							name="bn-locale"
							class="field__input"
							placeholder="en, fr, en-US, …"
							bind:value={locale}
							disabled={mode === 'edit'}
						/>
					</div>
				</div>
				<div class="grid-2">
					<div class="field">
						<label class="field__label" for="bn-layout">Layout</label>
						<select id="bn-layout" name="bn-layout" class="field__input" bind:value={layout}>
							<option value="bar">Bar</option>
							<option value="modal">Modal</option>
							<option value="corner">Corner</option>
						</select>
					</div>
					<div class="field">
						<label class="field__label" for="bn-position">Position</label>
						<select id="bn-position" name="bn-position" class="field__input" bind:value={position}>
							<option value="top">Top</option>
							<option value="bottom">Bottom</option>
							<option value="bottom-left">Bottom left</option>
							<option value="bottom-right">Bottom right</option>
							<option value="center">Center</option>
						</select>
					</div>
				</div>
				<div class="field">
					<label class="check-row" for="bn-active">
						<input id="bn-active" name="bn-active" type="checkbox" bind:checked={isActive} />
						<span>Active (serve as live banner for this region/locale)</span>
					</label>
				</div>
				<div class="field">
					<label class="field__label" for="bn-title">Title</label>
					<input
						id="bn-title"
						name="bn-title"
						class="field__input"
						placeholder="We respect your privacy"
						bind:value={copyTitle}
					/>
				</div>
				<div class="field">
					<label class="field__label" for="bn-body">Body</label>
					<textarea
						id="bn-body"
						name="bn-body"
						class="field__input field__input--tall"
						rows={5}
						bind:value={copyBody}
					></textarea>
				</div>
				<div class="grid-2">
					<div class="field">
						<label class="field__label" for="bn-accept">Accept-all label</label>
						<input
							id="bn-accept"
							name="bn-accept"
							class="field__input"
							bind:value={copyAcceptAll}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="bn-reject">Reject-all label</label>
						<input
							id="bn-reject"
							name="bn-reject"
							class="field__input"
							bind:value={copyRejectAll}
						/>
					</div>
				</div>
				<div class="grid-2">
					<div class="field">
						<label class="field__label" for="bn-cust">Customize label</label>
						<input
							id="bn-cust"
							name="bn-cust"
							class="field__input"
							bind:value={copyCustomize}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="bn-save">Save-preferences label</label>
						<input
							id="bn-save"
							name="bn-save"
							class="field__input"
							bind:value={copySavePreferences}
						/>
					</div>
				</div>
				<div class="grid-2">
					<div class="field">
						<label class="field__label" for="bn-priv-href">Privacy URL</label>
						<input
							id="bn-priv-href"
							name="bn-priv-href"
							class="field__input"
							bind:value={copyPrivacyHref}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="bn-priv-label">Privacy link label</label>
						<input
							id="bn-priv-label"
							name="bn-priv-label"
							class="field__input"
							bind:value={copyPrivacyLabel}
						/>
					</div>
				</div>
				<div class="form__actions">
					<button class="btn btn--primary" type="button" disabled={formBusy} onclick={save}>
						<CheckCircleIcon size={16} weight="bold" />
						<span>{formBusy ? 'Saving…' : mode === 'create' ? 'Create' : 'Save'}</span>
					</button>
					<button class="btn btn--secondary" type="button" onclick={closeDrawer}>
						<XIcon size={16} weight="bold" />
						<span>Cancel</span>
					</button>
				</div>
			</div>
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

	.toast, .error { display: flex; align-items: center; gap: 0.5rem; padding: 0.75rem 1rem; border-radius: var(--radius-2xl); font-size: 0.875rem; margin-bottom: 1rem; }
	.toast { background: rgba(15, 164, 175, 0.12); border: 1px solid rgba(15, 164, 175, 0.25); color: #5eead4; }
	.error { background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); color: #fca5a5; }

	.state { display: flex; align-items: center; justify-content: center; gap: 0.75rem; padding: 4rem 0; color: var(--color-grey-400); font-size: 0.875rem; }
	.state__spinner { width: 1.25rem; height: 1.25rem; border: 2px solid rgba(255, 255, 255, 0.1); border-top-color: var(--color-teal); border-radius: 50%; animation: spin 0.7s linear infinite; }
	@keyframes spin { to { transform: rotate(360deg); } }

	.empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 3rem 1rem; background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px); border: 1px dashed rgba(255, 255, 255, 0.1); border-radius: var(--radius-2xl); color: var(--color-grey-500); text-align: center; }
	.empty :global(svg) { color: var(--color-grey-500); }
	.empty__title { margin: 0.5rem 0 0; font-size: 1rem; font-weight: 600; color: var(--color-white); }
	.empty__sub { margin: 0 0 0.75rem; font-size: 0.875rem; color: var(--color-grey-400); }

	.grid { display: grid; gap: 0.85rem; grid-template-columns: 1fr; }
	.card { background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: var(--radius-2xl); padding: 1.25rem; box-shadow: 0 1px 0 rgba(255, 255, 255, 0.03) inset, 0 12px 32px rgba(0, 0, 0, 0.18); display: flex; flex-direction: column; gap: 0.75rem; }
	.card__head { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
	.card__eyebrow { font-size: 0.6875rem; font-weight: 700; color: var(--color-grey-500); text-transform: uppercase; letter-spacing: 0.06em; }
	.card__title { margin: 0; font-family: var(--font-heading); font-size: 1rem; font-weight: 600; color: var(--color-white); letter-spacing: -0.005em; line-height: 1.3; }
	.card__meta { display: grid; grid-template-columns: 5.5rem 1fr; gap: 0.35rem 0.75rem; font-size: 0.8125rem; color: var(--color-grey-200); margin: 0; }
	.card__meta dt { color: var(--color-grey-500); font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
	.card__meta dd { margin: 0; font-variant-numeric: tabular-nums; }
	.card__actions { display: flex; gap: 0.4rem; flex-wrap: wrap; margin-top: auto; padding-top: 0.5rem; }

	.pill { display: inline-flex; align-items: center; padding: 0.15rem 0.5rem; border-radius: var(--radius-full); font-size: 0.6875rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
	.pill--success { background: rgba(15, 164, 175, 0.12); color: #5eead4; }
	.pill--neutral { background: rgba(255, 255, 255, 0.06); color: var(--color-grey-300); }

	.btn { display: inline-flex; align-items: center; gap: 0.5rem; min-height: 3rem; padding: 0 1.25rem; border-radius: var(--radius-2xl); font-size: 0.8125rem; font-weight: 600; border: 1px solid transparent; background: transparent; color: var(--color-grey-300); cursor: pointer; transition: background-color 150ms, border-color 150ms, color 150ms, box-shadow 150ms, transform 150ms; }
	.btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn--primary { background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94)); color: var(--color-white); box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45); }
	.btn--primary:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55); }
	.btn--secondary { background: rgba(255, 255, 255, 0.05); border-color: rgba(255, 255, 255, 0.1); color: var(--color-grey-200); }
	.btn--secondary:hover:not(:disabled) { background: rgba(255, 255, 255, 0.1); border-color: rgba(255, 255, 255, 0.18); color: var(--color-white); }
	.btn--small { min-height: 2.5rem; padding: 0 0.65rem; font-size: 0.75rem; }

	.drawer-backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); z-index: 60; }
	.drawer { position: fixed; top: 0; right: 0; bottom: 0; width: min(640px, 92vw); background: var(--color-navy); border-left: 1px solid rgba(255, 255, 255, 0.08); padding: 1.5rem; overflow-y: auto; z-index: 70; box-shadow: -8px 0 24px rgba(0, 0, 0, 0.3); }
	.drawer__header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
	.drawer__title { font-family: var(--font-heading); font-size: 1rem; font-weight: 600; color: var(--color-white); margin: 0; letter-spacing: -0.01em; }
	.form { display: flex; flex-direction: column; gap: 0.85rem; }
	.form__actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.5rem; }
	.grid-2 { display: grid; grid-template-columns: 1fr; gap: 0.85rem; }
	.field { display: flex; flex-direction: column; gap: 0.4rem; }
	.field__label { font-size: 0.75rem; color: var(--color-grey-300); font-weight: 500; }
	.field__input { min-height: 3rem; padding: 0 1.25rem; background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); color: var(--color-white); border-radius: var(--radius-2xl); font-size: 0.875rem; width: 100%; font-family: inherit; color-scheme: dark; transition: border-color 150ms, box-shadow 150ms; }
	.field__input--tall { min-height: 6rem; }
	.field__input::placeholder { color: var(--color-grey-500); }
	.field__input:focus { outline: none; border-color: var(--color-teal); box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15); }
	.field__input:disabled { opacity: 0.55; cursor: not-allowed; }
	.check-row { display: inline-flex; gap: 0.5rem; align-items: center; font-size: 0.875rem; color: var(--color-grey-200); cursor: pointer; }
	.check-row input { accent-color: var(--color-teal); }

	@media (min-width: 480px) {
		.grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
		.grid-2 { grid-template-columns: 1fr 1fr; }
	}
	@media (min-width: 1024px) {
		.grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
	}
</style>
