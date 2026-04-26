<!--
  Phase 2.3 — Consent categories CRUD. The `necessary` category is
  backend-protected (POST/PUT throw BadRequest); we surface it read-only with
  a lock badge.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import PencilIcon from 'phosphor-svelte/lib/PencilIcon';
	import LockIcon from 'phosphor-svelte/lib/LockIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import { ApiError } from '$lib/api/client';
	import {
		listCategories,
		createCategory,
		updateCategory,
		type AdminCategory,
		type CategoryUpsertBody
	} from '$lib/api/admin-consent';

	type DrawerMode = 'create' | 'edit' | null;

	let categories = $state<AdminCategory[]>([]);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let mode = $state<DrawerMode>(null);
	let editingKey = $state<string | null>(null);

	let formKey = $state('');
	let formLabel = $state('');
	let formDescription = $state('');
	let formRequired = $state(false);
	let formSortOrder = $state(0);
	let formBusy = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			categories = await listCategories();
		} catch (e) {
			error =
				e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load categories';
		} finally {
			loading = false;
		}
	}

	function openCreate() {
		formKey = '';
		formLabel = '';
		formDescription = '';
		formRequired = false;
		formSortOrder = (categories.length + 1) * 10;
		editingKey = null;
		mode = 'create';
	}

	function openEdit(c: AdminCategory) {
		formKey = c.key;
		formLabel = c.label;
		formDescription = c.description;
		formRequired = c.is_required;
		formSortOrder = c.sort_order;
		editingKey = c.key;
		mode = 'edit';
	}

	function closeDrawer() {
		mode = null;
		editingKey = null;
	}

	async function save() {
		if (!formLabel.trim()) {
			error = 'Label is required';
			return;
		}
		formBusy = true;
		error = '';
		const body: CategoryUpsertBody = {
			key: formKey.trim(),
			label: formLabel.trim(),
			description: formDescription.trim(),
			is_required: formRequired,
			sort_order: formSortOrder
		};
		try {
			if (mode === 'create') {
				if (!formKey.trim()) {
					error = 'Key is required';
					formBusy = false;
					return;
				}
				await createCategory(body);
				flash('Category created');
			} else if (editingKey) {
				await updateCategory(editingKey, body);
				flash('Category updated');
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
	<title>Consent categories · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-consent-categories">
	<header class="page__header">
		<div class="page__title-row">
			<ListChecksIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Governance / Consent</span>
				<h1 class="page__title">Categories</h1>
				<p class="page__subtitle">
					Categories grouped under the consent banner. The <code>necessary</code> category is
					locked — it always reflects the strictly-required cookies and cannot be opted out
					of.
				</p>
			</div>
		</div>
		<div class="page__actions">
			<button class="btn btn--primary" type="button" onclick={openCreate}>
				<PlusIcon size={16} weight="bold" />
				<span>New category</span>
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
			<span>Loading categories…</span>
		</div>
	{:else if categories.length === 0}
		<div class="empty">
			<ListChecksIcon size={48} weight="duotone" />
			<p class="empty__title">No categories yet</p>
			<p class="empty__sub">Create one to start grouping services.</p>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Key</th>
							<th scope="col">Label</th>
							<th scope="col">Description</th>
							<th scope="col">Required</th>
							<th scope="col">Order</th>
							<th scope="col" class="table__actions-th" aria-label="Actions"></th>
						</tr>
					</thead>
					<tbody>
						{#each categories as c (c.key)}
							{@const locked = c.key === 'necessary'}
							<tr>
								<td>
									<code class="key">{c.key}</code>
									{#if locked}
										<span class="pill pill--neutral lock">
											<LockIcon size={11} weight="bold" />
											<span>Protected</span>
										</span>
									{/if}
								</td>
								<td>{c.label}</td>
								<td class="desc">{c.description}</td>
								<td>
									<span
										class={c.is_required
											? 'pill pill--warn'
											: 'pill pill--neutral'}
									>
										{c.is_required ? 'Yes' : 'No'}
									</span>
								</td>
								<td class="num">{c.sort_order}</td>
								<td class="row-actions">
									<button
										class="btn btn--secondary btn--small"
										type="button"
										onclick={() => openEdit(c)}
										disabled={locked}
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
		<aside class="drawer" aria-label="Category editor">
			<header class="drawer__header">
				<h2 class="drawer__title">
					{mode === 'create' ? 'New category' : 'Edit category'}
				</h2>
				<button class="btn btn--secondary btn--small" type="button" onclick={closeDrawer}>
					<XIcon size={14} weight="bold" />
					<span>Close</span>
				</button>
			</header>
			<div class="form">
				<div class="field">
					<label class="field__label" for="cat-key">Key</label>
					<input
						id="cat-key"
						name="cat-key"
						class="field__input"
						placeholder="analytics, marketing, …"
						bind:value={formKey}
						disabled={mode === 'edit'}
					/>
				</div>
				<div class="field">
					<label class="field__label" for="cat-label">Label</label>
					<input
						id="cat-label"
						name="cat-label"
						class="field__input"
						placeholder="Analytics"
						bind:value={formLabel}
					/>
				</div>
				<div class="field">
					<label class="field__label" for="cat-desc">Description</label>
					<textarea
						id="cat-desc"
						name="cat-desc"
						class="field__input field__input--tall"
						rows={4}
						bind:value={formDescription}
					></textarea>
				</div>
				<div class="grid-2">
					<div class="field">
						<label class="check-row" for="cat-required">
							<input
								id="cat-required"
								name="cat-required"
								type="checkbox"
								bind:checked={formRequired}
							/>
							<span>Required</span>
						</label>
					</div>
					<div class="field">
						<label class="field__label" for="cat-order">Sort order</label>
						<input
							id="cat-order"
							name="cat-order"
							type="number"
							class="field__input"
							bind:value={formSortOrder}
						/>
					</div>
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
	.page__subtitle code {
		font-size: 0.85em;
		padding: 0.1em 0.35em;
		border-radius: 0.25rem;
		background: rgba(255, 255, 255, 0.06);
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
		margin: 0;
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
	.num {
		font-variant-numeric: tabular-nums;
		text-align: right;
		color: var(--color-grey-300);
	}
	.key {
		color: var(--color-teal-light);
		margin-right: 0.4rem;
	}
	.desc {
		color: var(--color-grey-400);
		max-width: 32ch;
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
	.pill--warn {
		background: rgba(245, 158, 11, 0.12);
		color: #fcd34d;
	}
	.pill--neutral {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}
	.lock {
		gap: 0.25rem;
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
		padding-top: 1.5rem;
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
