<!--
  Phase 2.3 — Consent services CRUD. A service row tells the consent gate
  "when category X is granted, allow this vendor". Grouped by category in
  the table view.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import WrenchIcon from 'phosphor-svelte/lib/WrenchIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import PencilIcon from 'phosphor-svelte/lib/PencilIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import { ApiError } from '$lib/api/client';
	import {
		listServices,
		createService,
		updateService,
		listCategories,
		type AdminCategory,
		type AdminService,
		type ServiceUpsertBody
	} from '$lib/api/admin-consent';

	type DrawerMode = 'create' | 'edit' | null;

	let services = $state<AdminService[]>([]);
	let categories = $state<AdminCategory[]>([]);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let mode = $state<DrawerMode>(null);
	let editingId = $state<string | null>(null);

	let formSlug = $state('');
	let formName = $state('');
	let formVendor = $state('');
	let formCategory = $state('analytics');
	let formDomains = $state('');
	let formPrivacyUrl = $state('');
	let formDescription = $state('');
	let formIsActive = $state(true);
	let formBusy = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			[services, categories] = await Promise.all([listServices(), listCategories()]);
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load services';
		} finally {
			loading = false;
		}
	}

	function openCreate() {
		formSlug = '';
		formName = '';
		formVendor = '';
		formCategory = categories[0]?.key ?? 'analytics';
		formDomains = '';
		formPrivacyUrl = '';
		formDescription = '';
		formIsActive = true;
		editingId = null;
		mode = 'create';
	}

	function openEdit(s: AdminService) {
		formSlug = s.slug;
		formName = s.name;
		formVendor = s.vendor;
		formCategory = s.category;
		formDomains = s.domains.join(', ');
		formPrivacyUrl = s.privacy_url ?? '';
		formDescription = s.description ?? '';
		formIsActive = s.is_active;
		editingId = s.id;
		mode = 'edit';
	}

	function closeDrawer() {
		mode = null;
		editingId = null;
	}

	async function save() {
		if (!formName.trim() || !formVendor.trim() || !formCategory.trim()) {
			error = 'Name, vendor, and category are required';
			return;
		}
		formBusy = true;
		error = '';
		const domains = formDomains
			.split(',')
			.map((d) => d.trim())
			.filter((d) => d.length > 0);
		const body: ServiceUpsertBody = {
			slug: formSlug.trim(),
			name: formName.trim(),
			vendor: formVendor.trim(),
			category: formCategory.trim(),
			domains,
			privacy_url: formPrivacyUrl.trim() || null,
			description: formDescription.trim() || null,
			is_active: formIsActive
		};
		try {
			if (mode === 'create') {
				if (!formSlug.trim()) {
					error = 'Slug is required';
					formBusy = false;
					return;
				}
				await createService(body);
				flash('Service created');
			} else if (editingId) {
				await updateService(editingId, body);
				flash('Service updated');
			}
			closeDrawer();
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Save failed';
		} finally {
			formBusy = false;
		}
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Consent services · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-consent-services">
	<header class="page__header">
		<div class="page__title-row">
			<WrenchIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Governance / Consent</span>
				<h1 class="page__title">Services</h1>
				<p class="page__subtitle">
					Third-party scripts and SDKs grouped under a consent category. The renderer
					keeps a service inactive until its category is granted by the visitor.
				</p>
			</div>
		</div>
		<div class="page__actions">
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New service</span>
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
			<span>Loading services…</span>
		</div>
	{:else if services.length === 0}
		<div class="empty">
			<WrenchIcon size={48} weight="duotone" />
			<p class="empty__title">No services configured</p>
			<p class="empty__sub">
				Add Google Analytics, Meta Pixel, Stripe, etc. once categories exist.
			</p>
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New service</span>
			</button>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Slug</th>
							<th scope="col">Name</th>
							<th scope="col">Vendor</th>
							<th scope="col">Category</th>
							<th scope="col">Domains</th>
							<th scope="col">Status</th>
							<th scope="col" class="table__actions-th" aria-label="Actions"></th>
						</tr>
					</thead>
					<tbody>
						{#each services as s (s.id)}
							<tr>
								<td><code class="key">{s.slug}</code></td>
								<td>{s.name}</td>
								<td>{s.vendor}</td>
								<td><span class="pill pill--neutral">{s.category}</span></td>
								<td class="domains">
									{#if s.domains.length === 0}—{:else}
										<code>{s.domains.slice(0, 2).join(', ')}</code>
										{#if s.domains.length > 2}<span class="muted"
												>+{s.domains.length - 2}</span
											>{/if}
									{/if}
								</td>
								<td>
									<span
										class={s.is_active
											? 'pill pill--success'
											: 'pill pill--neutral'}
									>
										{s.is_active ? 'Active' : 'Disabled'}
									</span>
								</td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => openEdit(s)}
										aria-label="Edit"
									>
										<PencilIcon size={14} weight="bold" />
										<span>Edit</span>
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
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
		<aside class="drawer" aria-label="Service editor">
			<header class="drawer__header">
				<h2 class="drawer__title">
					{mode === 'create' ? 'New service' : 'Edit service'}
				</h2>
				<button class="btn btn--secondary btn--small" type="button" onclick={closeDrawer}>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
				</button>
			</header>
			<div class="form">
				<div class="grid-2">
					<div class="field">
						<label class="field__label" for="svc-slug">Slug</label>
						<input
							id="svc-slug"
							name="svc-slug"
							class="field__input"
							placeholder="ga4, meta-pixel, …"
							bind:value={formSlug}
							disabled={mode === 'edit'}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="svc-category">Category</label>
						<select
							id="svc-category"
							name="svc-category"
							class="field__input"
							bind:value={formCategory}
						>
							{#each categories as c (c.key)}
								<option value={c.key}>{c.label} ({c.key})</option>
							{/each}
						</select>
					</div>
				</div>
				<div class="grid-2">
					<div class="field">
						<label class="field__label" for="svc-name">Name</label>
						<input
							id="svc-name"
							name="svc-name"
							class="field__input"
							placeholder="Google Analytics 4"
							bind:value={formName}
						/>
					</div>
					<div class="field">
						<label class="field__label" for="svc-vendor">Vendor</label>
						<input
							id="svc-vendor"
							name="svc-vendor"
							class="field__input"
							placeholder="Google LLC"
							bind:value={formVendor}
						/>
					</div>
				</div>
				<div class="field">
					<label class="field__label" for="svc-domains">Domains (comma-separated)</label>
					<input
						id="svc-domains"
						name="svc-domains"
						class="field__input"
						placeholder="www.google-analytics.com, region1.analytics.google.com"
						bind:value={formDomains}
					/>
				</div>
				<div class="field">
					<label class="field__label" for="svc-privacy">Privacy URL</label>
					<input
						id="svc-privacy"
						name="svc-privacy"
						type="url"
						class="field__input"
						placeholder="https://policies.google.com/privacy"
						bind:value={formPrivacyUrl}
					/>
				</div>
				<div class="field">
					<label class="field__label" for="svc-desc">Description</label>
					<textarea
						id="svc-desc"
						name="svc-desc"
						class="field__input field__input--tall"
						rows={4}
						bind:value={formDescription}
					></textarea>
				</div>
				<div class="field">
					<label class="check-row" for="svc-active">
						<input
							id="svc-active"
							name="svc-active"
							type="checkbox"
							bind:checked={formIsActive}
						/>
						<span>Enabled</span>
					</label>
				</div>
				<div class="form__actions">
					<button
						class="btn btn--primary"
						type="button"
						disabled={formBusy}
						onclick={save}
					>
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

	.toast,
	.error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		margin-bottom: 1rem;
	}
	.toast {
		background: rgba(15, 164, 175, 0.12);
		border: 1px solid rgba(15, 164, 175, 0.25);
		color: #5eead4;
	}
	.error {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
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
		margin: 0 0 0.75rem;
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
	.key {
		color: var(--color-teal-light);
	}
	.muted {
		color: var(--color-grey-500);
		margin-left: 0.35rem;
		font-size: 0.75rem;
	}
	.domains {
		max-width: 24ch;
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
	.pill--neutral {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
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
		width: min(640px, 92vw);
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
	.form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.form__actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		margin-top: 0.5rem;
	}
	.grid-2 {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.85rem;
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
	.field__input--tall {
		min-height: 5rem;
	}
	.field__input::placeholder {
		color: var(--color-grey-500);
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}
	.field__input:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}
	.check-row {
		display: inline-flex;
		gap: 0.5rem;
		align-items: center;
		font-size: 0.875rem;
		color: var(--color-grey-200);
		cursor: pointer;
	}
	.check-row input {
		accent-color: var(--color-teal);
	}

	@media (min-width: 480px) {
		.grid-2 {
			grid-template-columns: 1fr 1fr;
		}
	}
</style>
