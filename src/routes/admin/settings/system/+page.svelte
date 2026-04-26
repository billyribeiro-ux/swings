<script lang="ts">
	import { onMount } from 'svelte';
	import GearIcon from 'phosphor-svelte/lib/GearIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import ArrowClockwiseIcon from 'phosphor-svelte/lib/ArrowClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import EyeSlashIcon from 'phosphor-svelte/lib/EyeSlashIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import { ApiError } from '$lib/api/client';
	import { appSettings, type SettingType, type SettingView } from '$lib/api/admin-security';

	let settings = $state<SettingView[]>([]);
	let loading = $state(true);
	let saving = $state<Record<string, boolean>>({});
	let error = $state('');
	let toast = $state('');
	let revealCache = $state<Record<string, string>>({});
	let drafts = $state<Record<string, string>>({});

	let createKey = $state('');
	let createType = $state<SettingType>('string');
	let createSecret = $state(false);
	let createCategory = $state('general');
	let createValue = $state('');
	let createDesc = $state('');
	let createBusy = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			const res = await appSettings.list();
			settings = res.data;
			drafts = {};
			revealCache = {};
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load settings';
		} finally {
			loading = false;
		}
	}

	function categories(): string[] {
		const set = new Set(settings.map((s) => s.category));
		return Array.from(set).sort();
	}

	function rowsByCategory(cat: string): SettingView[] {
		return settings.filter((s) => s.category === cat);
	}

	function draftValue(s: SettingView): string {
		if (s.key in drafts) return drafts[s.key]!;
		if (s.is_secret) return revealCache[s.key] ?? '';
		if (s.value_type === 'json') {
			try {
				return JSON.stringify(s.value, null, 2);
			} catch {
				return String(s.value);
			}
		}
		return String(s.value ?? '');
	}

	function setDraft(key: string, val: string) {
		drafts = { ...drafts, [key]: val };
	}

	function isDirty(s: SettingView): boolean {
		return s.key in drafts;
	}

	function parseDraft(s: SettingView, raw: string): unknown {
		switch (s.value_type) {
			case 'int': {
				const n = Number(raw);
				if (!Number.isInteger(n)) throw new Error('Expected integer');
				return n;
			}
			case 'bool':
				if (raw === 'true') return true;
				if (raw === 'false') return false;
				throw new Error('Expected true|false');
			case 'json':
				return JSON.parse(raw);
			case 'secret':
			case 'string':
			default:
				return raw;
		}
	}

	async function save(s: SettingView) {
		const raw = drafts[s.key];
		if (raw === undefined) return;
		saving = { ...saving, [s.key]: true };
		error = '';
		try {
			const value = parseDraft(s, raw);
			await appSettings.upsert(s.key, { value });
			flash(`Saved ${s.key}`);
			await refresh();
		} catch (e) {
			if (e instanceof ApiError) error = `${e.status}: ${e.message}`;
			else if (e instanceof Error) error = `${s.key}: ${e.message}`;
			else error = `Save failed for ${s.key}`;
		} finally {
			saving = { ...saving, [s.key]: false };
		}
	}

	function discard(s: SettingView) {
		const next = { ...drafts };
		delete next[s.key];
		drafts = next;
	}

	async function reveal(s: SettingView) {
		if (revealCache[s.key]) {
			const next = { ...revealCache };
			delete next[s.key];
			revealCache = next;
			return;
		}
		try {
			const res = await appSettings.get(s.key, true);
			const v = (res.revealed_value ?? '') as unknown;
			revealCache = {
				...revealCache,
				[s.key]: typeof v === 'string' ? v : JSON.stringify(v)
			};
		} catch (e) {
			error = e instanceof ApiError ? `Reveal denied (${e.status})` : 'Reveal failed';
		}
	}

	async function reload() {
		try {
			const res = await appSettings.reload();
			flash(`Hot-reloaded ${res.reloaded} settings`);
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Reload failed';
		}
	}

	async function create(e: Event) {
		e.preventDefault();
		if (!createKey.trim()) return;
		createBusy = true;
		error = '';
		try {
			let value: unknown = createValue;
			if (createType === 'int') value = Number(createValue);
			else if (createType === 'bool') value = createValue === 'true';
			else if (createType === 'json') value = JSON.parse(createValue || 'null');
			await appSettings.upsert(createKey.trim(), {
				value,
				value_type: createType,
				is_secret: createSecret,
				category: createCategory.trim() || 'general',
				description: createDesc.trim() || undefined
			});
			flash(`Created ${createKey}`);
			createKey = '';
			createValue = '';
			createDesc = '';
			createSecret = false;
			await refresh();
		} catch (e) {
			if (e instanceof ApiError) error = `${e.status}: ${e.message}`;
			else if (e instanceof Error) error = e.message;
			else error = 'Create failed';
		} finally {
			createBusy = false;
		}
	}

	function maintenanceCheckbox(s: SettingView) {
		const v = String(s.value);
		return v === 'true';
	}

	async function toggleMaintenance(s: SettingView) {
		const next = !maintenanceCheckbox(s);
		try {
			await appSettings.upsert(s.key, { value: next });
			flash(next ? 'Maintenance mode ON' : 'Maintenance mode OFF');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Toggle failed';
		}
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>System settings · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-settings-system-page">
	<header class="page__header">
		<div class="page__title-row">
			<GearIcon size={28} weight="duotone" />
			<h1 class="page__title">System settings</h1>
		</div>
		<p class="page__subtitle">
			Typed key/value catalogue backed by <code>app_settings</code>. Secrets are encrypted at
			rest and reveal requires <code>admin.settings.read_secret</code>. Mutations hot-reload
			the in-process snapshot used by the maintenance-mode middleware.
		</p>
	</header>

	{#if toast}
		<div class="toast">{toast}</div>
	{/if}
	{#if error}
		<div class="error" role="alert" data-testid="settings-error">{error}</div>
	{/if}

	<div class="actions">
		<button class="btn btn--ghost" onclick={refresh}>
			<ArrowClockwiseIcon size={16} weight="bold" />
			Refresh
		</button>
		<button class="btn btn--ghost" onclick={reload} title="Reload in-memory snapshot">
			Hot-reload cache
		</button>
	</div>

	{#if loading}
		<p class="muted">Loading…</p>
	{:else}
		{#each categories() as cat (cat)}
			<section class="section card">
				<h2 class="section__title">{cat}</h2>
				<div class="rows">
					{#each rowsByCategory(cat) as s (s.key)}
						<div class="row" data-testid={`setting-row-${s.key}`}>
							<div class="row__meta">
								<code class="row__key">{s.key}</code>
								<span class="type-pill type-pill--{s.value_type}">
									{s.value_type}
								</span>
								{#if s.is_secret}
									<span class="type-pill type-pill--secret">secret</span>
								{/if}
								{#if s.description}
									<p class="row__desc">{s.description}</p>
								{/if}
								<p class="row__updated">
									Updated {new Date(s.updated_at).toLocaleString()}
								</p>
							</div>
							<div class="row__editor">
								{#if s.key === 'system.maintenance_mode' && s.value_type === 'bool'}
									<label class="switch">
										<input
											type="checkbox"
											checked={maintenanceCheckbox(s)}
											onchange={() => toggleMaintenance(s)}
											data-testid="maintenance-toggle"
										/>
										<span>{maintenanceCheckbox(s) ? 'ON' : 'OFF'}</span>
									</label>
								{:else if s.value_type === 'bool'}
									<select
										class="field__input"
										value={draftValue(s)}
										onchange={(e) =>
											setDraft(
												s.key,
												(e.currentTarget as HTMLSelectElement).value
											)}
									>
										<option value="true">true</option>
										<option value="false">false</option>
									</select>
								{:else if s.value_type === 'json'}
									<textarea
										class="field__input field__input--mono"
										rows="6"
										value={draftValue(s)}
										oninput={(e) =>
											setDraft(
												s.key,
												(e.currentTarget as HTMLTextAreaElement).value
											)}
									></textarea>
								{:else if s.is_secret}
									<div class="secret-row">
										<input
											class="field__input"
											type={revealCache[s.key] ? 'text' : 'password'}
											placeholder={revealCache[s.key]
												? ''
												: '*** redacted ***'}
											value={draftValue(s)}
											oninput={(e) =>
												setDraft(
													s.key,
													(e.currentTarget as HTMLInputElement).value
												)}
										/>
										<button
											class="btn btn--ghost btn--small"
											type="button"
											onclick={() => reveal(s)}
											title={revealCache[s.key]
												? 'Hide'
												: 'Reveal current value'}
										>
											{#if revealCache[s.key]}
												<EyeSlashIcon size={14} />
											{:else}
												<EyeIcon size={14} />
											{/if}
										</button>
									</div>
								{:else}
									<input
										class="field__input"
										value={draftValue(s)}
										oninput={(e) =>
											setDraft(
												s.key,
												(e.currentTarget as HTMLInputElement).value
											)}
									/>
								{/if}
								<div class="row__actions">
									<button
										class="btn btn--ghost btn--small"
										onclick={() => discard(s)}
										disabled={!isDirty(s)}
									>
										Discard
									</button>
									<button
										class="btn btn--primary btn--small"
										onclick={() => save(s)}
										disabled={!isDirty(s) || saving[s.key]}
										data-testid={`save-${s.key}`}
									>
										<FloppyDiskIcon size={14} weight="bold" />
										{saving[s.key] ? 'Saving…' : 'Save'}
									</button>
								</div>
							</div>
						</div>
					{/each}
				</div>
			</section>
		{/each}
	{/if}

	<section class="section card card--create">
		<h2 class="section__title">
			<PlusIcon size={18} weight="bold" />
			Create setting
		</h2>
		<form class="create-form" onsubmit={create}>
			<div class="field">
				<label class="field__label" for="ckey">Key</label>
				<input
					id="ckey"
					class="field__input"
					placeholder="namespace.key"
					bind:value={createKey}
					data-testid="create-key"
					required
				/>
			</div>
			<div class="field">
				<label class="field__label" for="ctype">Type</label>
				<select id="ctype" class="field__input" bind:value={createType}>
					<option value="string">string</option>
					<option value="int">int</option>
					<option value="bool">bool</option>
					<option value="json">json</option>
					<option value="secret">secret</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="ccat">Category</label>
				<input
					id="ccat"
					class="field__input"
					placeholder="general"
					bind:value={createCategory}
				/>
			</div>
			<div class="field">
				<label class="field__label" for="cval">Value</label>
				<input
					id="cval"
					class="field__input"
					placeholder="value (json text for json type)"
					bind:value={createValue}
					data-testid="create-value"
				/>
			</div>
			<div class="field field--wide">
				<label class="field__label" for="cdesc">Description</label>
				<input
					id="cdesc"
					class="field__input"
					placeholder="Optional admin-facing note"
					bind:value={createDesc}
				/>
			</div>
			<label class="field field--checkbox">
				<input type="checkbox" bind:checked={createSecret} />
				<span>Encrypt at rest (secret)</span>
			</label>
			<button class="btn btn--primary" type="submit" disabled={createBusy}>
				{createBusy ? 'Creating…' : 'Create'}
			</button>
		</form>
	</section>
</div>

<style>
	.page {
		max-width: 1100px;
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
	.toast {
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.actions {
		display: flex;
		gap: var(--space-2);
		margin-bottom: var(--space-4);
	}
	.section {
		margin-bottom: var(--space-5);
	}
	.section__title {
		font-size: var(--fs-md);
		font-family: var(--font-heading);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin: 0 0 var(--space-3) 0;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		text-transform: capitalize;
	}
	.card {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: var(--space-5);
	}
	.card--create {
		border-style: dashed;
	}
	.rows {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.row {
		display: grid;
		grid-template-columns: 1fr 1.5fr;
		gap: var(--space-4);
		padding-bottom: var(--space-4);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.row:last-child {
		border-bottom: 0;
	}
	@media (max-width: 720px) {
		.row {
			grid-template-columns: 1fr;
		}
	}
	.row__meta {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.row__key {
		font-size: var(--fs-sm);
		color: var(--color-white);
	}
	.row__desc {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin: var(--space-1) 0 0;
	}
	.row__updated {
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
		margin: var(--space-1) 0 0;
	}
	.type-pill {
		display: inline-block;
		padding: 0.05rem 0.4rem;
		font-size: 0.65rem;
		text-transform: uppercase;
		border-radius: var(--radius-full);
		letter-spacing: 0.04em;
		margin-right: var(--space-1);
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
	.type-pill--secret {
		background: rgba(245, 158, 11, 0.15);
		color: #fbbf24;
	}
	.type-pill--json {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}
	.row__editor {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.row__actions {
		display: flex;
		gap: var(--space-2);
		justify-content: flex-end;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field--checkbox {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
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
		border-radius: var(--radius-2xl);
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
	.secret-row {
		display: flex;
		gap: var(--space-2);
		align-items: stretch;
	}
	.secret-row .field__input {
		flex: 1;
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-2xl);
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
	.create-form {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: var(--space-3);
		align-items: end;
	}
	.switch {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
	}
</style>
