<!--
  EC-01 admin products — list + create.

  Wave-2 polish: enterprise-grade dashboard styling. Uses PE7 shared
  primitives (Button, Dialog, FormField); no Tailwind, no Lucide. Icons
  come from `@iconify-json/ph` via phosphor-svelte (per PE7 convention).
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { Button, Dialog, FormField } from '$lib/components/shared';
	import { productsApi } from '$lib/api/products';
	import type {
		Product,
		ProductStatus,
		ProductType,
		CreateProductRequest
	} from '$lib/api/products';
	import { ApiError } from '$lib/api/client';
	import { toast } from '$lib/stores/toast.svelte';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import PackageIcon from 'phosphor-svelte/lib/PackageIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import Tooltip from '$lib/components/ui/Tooltip.svelte';

	let rows = $state<Product[]>([]);
	let total = $state(0);
	let page = $state(1);
	let perPage = $state(20);
	let totalPages = $state(1);
	let loading = $state(true);
	let search = $state('');
	let statusFilter = $state<ProductStatus | ''>('');
	let typeFilter = $state<ProductType | ''>('');

	let createOpen = $state(false);
	let creating = $state(false);
	let createError = $state<string | null>(null);

	// Create form state.
	let cSlug = $state('');
	let cName = $state('');
	let cType = $state<ProductType>('simple');
	let cStatus = $state<ProductStatus>('draft');
	let cPriceDollars = $state('');
	let cDescription = $state('');

	const publishedCount = $derived(rows.filter((p) => p.status === 'published').length);
	const draftCount = $derived(rows.filter((p) => p.status === 'draft').length);

	async function load() {
		loading = true;
		try {
			const res = await productsApi.adminList({
				page,
				per_page: perPage,
				status: statusFilter === '' ? undefined : statusFilter,
				product_type: typeFilter === '' ? undefined : typeFilter,
				search: search.trim() || undefined
			});
			rows = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch (err) {
			toast.error('Failed to load products', {
				description: err instanceof Error ? err.message : undefined
			});
		} finally {
			loading = false;
		}
	}

	// One-shot initial load. The previous `$effect(() => load())` paired with
	// `bind:value={search}` re-fired the API on every keystroke (no debounce)
	// AND duplicated the `onchange` handlers below. `onMount` runs once;
	// `onchange={() => { page = 1; load(); }}` keeps the user-triggered loads.
	onMount(load);

	function openCreate() {
		cSlug = '';
		cName = '';
		cType = 'simple';
		cStatus = 'draft';
		cPriceDollars = '';
		cDescription = '';
		createError = null;
		createOpen = true;
	}

	async function submitCreate(e: SubmitEvent) {
		e.preventDefault();
		if (creating) return;
		creating = true;
		createError = null;
		try {
			const payload: CreateProductRequest = {
				slug: cSlug.trim(),
				name: cName.trim(),
				product_type: cType,
				status: cStatus,
				price_cents: cPriceDollars ? Math.round(Number(cPriceDollars) * 100) : null,
				description: cDescription.trim() || null
			};
			const created = await productsApi.adminCreate(payload);
			toast.success(`Product "${created.name}" created`);
			createOpen = false;
			goto(`/admin/products/${created.id}`);
		} catch (err) {
			createError = err instanceof ApiError ? err.message : 'Create failed';
			toast.error('Failed to create product', {
				description: err instanceof Error ? err.message : undefined
			});
		} finally {
			creating = false;
		}
	}

	function fmtMoney(cents: number | null | undefined, currency: string): string {
		if (cents === null || cents === undefined) return '—';
		const amount = (cents / 100).toFixed(2);
		return currency === 'USD' ? `$${amount}` : `${amount} ${currency}`;
	}

	function statusClass(status: string): string {
		return `pr-badge pr-badge--${status}`;
	}

	function typeLabel(t: string): string {
		return t.charAt(0).toUpperCase() + t.slice(1);
	}
</script>

<svelte:head>
	<title>Products - Admin</title>
</svelte:head>

{#snippet plusIcon()}<PlusIcon size={16} weight="bold" />{/snippet}

<div class="pr-admin">
	<header class="pr-admin__header">
		<div class="pr-admin__heading">
			<span class="pr-admin__eyebrow">Commerce</span>
			<h1 class="pr-admin__title">Products</h1>
			<p class="pr-admin__subtitle">
				Manage your catalog: simple goods, subscriptions, downloads, and bundles.
				{total} total · {publishedCount} published · {draftCount} draft.
			</p>
		</div>
		<Button onclick={openCreate} iconLeading={plusIcon}>New product</Button>
	</header>

	<div class="pr-filters">
		<div class="pr-filters__field pr-filters__field--search">
			<label class="pr-filters__label" for="pr-search">Search</label>
			<div class="pr-search-wrap">
				<MagnifyingGlassIcon size={16} weight="bold" class="pr-search-icon" />
				<input
					id="pr-search"
					name="pr-search"
					type="search"
					class="pr-input pr-input--search"
					placeholder="Search by name or slug…"
					bind:value={search}
					onchange={() => {
						page = 1;
						load();
					}}
				/>
			</div>
		</div>
		<div class="pr-filters__field">
			<label class="pr-filters__label" for="pr-status-filter">Status</label>
			<div class="pr-select-wrap">
				<select
					id="pr-status-filter"
					name="pr-status-filter"
					class="pr-input pr-input--select"
					bind:value={statusFilter}
					onchange={() => {
						page = 1;
						load();
					}}
				>
					<option value="">All statuses</option>
					<option value="draft">Draft</option>
					<option value="published">Published</option>
					<option value="archived">Archived</option>
				</select>
				<CaretDownIcon size={14} weight="bold" class="pr-select-caret" />
			</div>
		</div>
		<div class="pr-filters__field">
			<label class="pr-filters__label" for="pr-type-filter">Type</label>
			<div class="pr-select-wrap">
				<select
					id="pr-type-filter"
					name="pr-type-filter"
					class="pr-input pr-input--select"
					bind:value={typeFilter}
					onchange={() => {
						page = 1;
						load();
					}}
				>
					<option value="">All types</option>
					<option value="simple">Simple</option>
					<option value="subscription">Subscription</option>
					<option value="downloadable">Downloadable</option>
					<option value="bundle">Bundle</option>
				</select>
				<CaretDownIcon size={14} weight="bold" class="pr-select-caret" />
			</div>
		</div>
	</div>

	{#if loading}
		<div class="pr-admin__loading">Loading…</div>
	{:else if rows.length === 0}
		<div class="pr-admin__empty">
			<PackageIcon size={48} weight="duotone" />
			<h2 class="pr-admin__empty-title">No products yet</h2>
			<p class="pr-admin__empty-desc">
				Get started by creating your first product. You can configure pricing, type, and
				status in seconds.
			</p>
			<Button variant="secondary" onclick={openCreate} iconLeading={plusIcon}>
				Create your first product
			</Button>
		</div>
	{:else}
		<div class="pr-table-wrap">
			<table class="pr-table">
				<thead>
					<tr>
						<th>Name</th>
						<th>Slug</th>
						<th>Type</th>
						<th>Status</th>
						<th class="pr-table__right">Price</th>
						<th class="pr-table__right">Updated</th>
						<th class="pr-table__actions-h" aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each rows as product (product.id)}
						<tr>
							<td class="pr-table__name">{product.name}</td>
							<td class="pr-table__mono">{product.slug}</td>
							<td class="pr-table__type">{typeLabel(product.product_type)}</td>
							<td><span class={statusClass(product.status)}>{product.status}</span></td>
							<td class="pr-table__right pr-table__num">
								{fmtMoney(product.price_cents, product.currency)}
							</td>
							<td class="pr-table__right pr-table__date">
								{new Date(product.updated_at).toLocaleDateString()}
							</td>
							<td class="pr-table__actions">
								<Tooltip label="Edit {product.name}">
									<a
										href={`/admin/products/${product.id}`}
										class="pr-icon-btn"
										aria-label="Edit {product.name}"
									>
										<PencilSimpleIcon size={16} weight="bold" />
									</a>
								</Tooltip>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<nav class="pr-pagination" aria-label="Pagination">
				<button
					type="button"
					class="pr-pag-btn"
					disabled={page <= 1}
					onclick={() => {
						page = Math.max(1, page - 1);
					}}
				>
					<CaretLeftIcon size={14} weight="bold" />
					<span>Previous</span>
				</button>
				<span class="pr-pagination__info">Page {page} of {totalPages}</span>
				<button
					type="button"
					class="pr-pag-btn"
					disabled={page >= totalPages}
					onclick={() => {
						page = Math.min(totalPages, page + 1);
					}}
				>
					<span>Next</span>
					<CaretRightIcon size={14} weight="bold" />
				</button>
			</nav>
		{/if}
	{/if}
</div>

<Dialog bind:open={createOpen} title="Create product" size="md">
	<form class="pr-form" onsubmit={submitCreate}>
		{#if createError}
			<p class="pr-form__error" role="alert">{createError}</p>
		{/if}

		<FormField for="pr-name" label="Name" required>
			{#snippet children({ describedBy, invalid, required })}
				<input
					id="pr-name"
					name="pr-name"
					type="text"
					bind:value={cName}
					{required}
					aria-invalid={invalid}
					aria-describedby={describedBy}
				/>
			{/snippet}
		</FormField>

		<FormField
			for="pr-slug"
			label="Slug"
			description="Lowercase URL segment; must be unique."
			required
		>
			{#snippet children({ describedBy, invalid, required })}
				<input
					id="pr-slug"
					name="pr-slug"
					type="text"
					pattern="[a-z0-9-]+"
					bind:value={cSlug}
					{required}
					aria-invalid={invalid}
					aria-describedby={describedBy}
				/>
			{/snippet}
		</FormField>

		<div class="pr-form__grid">
			<FormField for="pr-type" label="Type" required>
				{#snippet children({ describedBy, invalid, required })}
					<select
						id="pr-type"
						name="pr-type"
						bind:value={cType}
						{required}
						aria-invalid={invalid}
						aria-describedby={describedBy}
					>
						<option value="simple">Simple</option>
						<option value="subscription">Subscription</option>
						<option value="downloadable">Downloadable</option>
						<option value="bundle">Bundle</option>
					</select>
				{/snippet}
			</FormField>

			<FormField for="pr-status" label="Status">
				{#snippet children({ describedBy, invalid })}
					<select
						id="pr-status"
						name="pr-status"
						bind:value={cStatus}
						aria-invalid={invalid}
						aria-describedby={describedBy}
					>
						<option value="draft">Draft</option>
						<option value="published">Published</option>
						<option value="archived">Archived</option>
					</select>
				{/snippet}
			</FormField>
		</div>

		<FormField
			for="pr-price"
			label="Price (USD)"
			description="Decimal dollars; stored as integer cents server-side."
		>
			{#snippet children({ describedBy, invalid })}
				<input
					id="pr-price"
					name="pr-price"
					type="number"
					step="0.01"
					min="0"
					bind:value={cPriceDollars}
					aria-invalid={invalid}
					aria-describedby={describedBy}
				/>
			{/snippet}
		</FormField>

		<FormField for="pr-desc" label="Description">
			{#snippet children({ describedBy, invalid })}
				<textarea
					id="pr-desc"
					name="pr-desc"
					rows={3}
					bind:value={cDescription}
					aria-invalid={invalid}
					aria-describedby={describedBy}
				></textarea>
			{/snippet}
		</FormField>

		<div class="pr-form__actions">
			<Button variant="secondary" onclick={() => (createOpen = false)}>Cancel</Button>
			<Button type="submit" loading={creating} disabled={creating}>Create</Button>
		</div>
	</form>
</Dialog>

<style>
	.pr-admin {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.pr-admin__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: flex-start;
	}

	.pr-admin__heading {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}

	.pr-admin__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.pr-admin__title {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		font-family: var(--font-heading);
		line-height: 1.2;
		letter-spacing: -0.01em;
		margin: 0;
	}

	.pr-admin__subtitle {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
	}

	/* ── Filters card ───────────────────── */
	.pr-filters {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.pr-filters__field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		min-width: 0;
	}

	.pr-filters__label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.pr-search-wrap,
	.pr-select-wrap {
		position: relative;
	}

	:global(.pr-search-icon) {
		position: absolute;
		left: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	:global(.pr-select-caret) {
		position: absolute;
		right: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	.pr-input {
		width: 100%;
		min-height: 3rem;
		padding: 0 1.25rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: 0.875rem;
		font-family: var(--font-ui);
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.pr-input::placeholder {
		color: var(--color-grey-500);
	}

	.pr-input:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.pr-input--search {
		padding-left: 2.25rem;
	}

	.pr-input--select {
		appearance: none;
		-webkit-appearance: none;
		-moz-appearance: none;
		padding-right: 2.25rem;
		cursor: pointer;
	}

	.pr-input--select option {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		color: var(--color-white);
	}

	/* ── Loading / empty ────────────────── */
	.pr-admin__loading {
		padding: 2.5rem 1rem;
		text-align: center;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-400);
		font-size: 0.875rem;
	}

	.pr-admin__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.85rem;
		text-align: center;
		padding: 3rem 1.5rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-500);
	}

	.pr-admin__empty :global(svg) {
		color: var(--color-grey-500);
	}

	.pr-admin__empty-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0;
	}

	.pr-admin__empty-desc {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 36ch;
		margin: 0;
		line-height: 1.55;
	}

	/* ── Table ──────────────────────────── */
	.pr-table-wrap {
		overflow-x: auto;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.pr-table {
		width: 100%;
		border-collapse: collapse;
		min-width: 720px;
	}

	.pr-table thead {
		background-color: rgba(255, 255, 255, 0.02);
	}

	.pr-table th {
		text-align: left;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.875rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		white-space: nowrap;
	}

	.pr-table td {
		padding: 0.875rem 1rem;
		font-size: 0.875rem;
		color: var(--color-grey-300);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		line-height: 1.45;
	}

	.pr-table tbody tr {
		transition: background-color 150ms var(--ease-out);
	}

	.pr-table tbody tr:hover {
		background-color: rgba(255, 255, 255, 0.02);
	}

	.pr-table tbody tr:last-child td {
		border-bottom: none;
	}

	.pr-table__name {
		font-weight: 600;
		color: var(--color-white);
	}

	.pr-table__mono {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--color-grey-400);
	}

	.pr-table__type {
		color: var(--color-grey-300);
	}

	.pr-table__right {
		text-align: right;
	}

	.pr-table__num {
		font-variant-numeric: tabular-nums;
		font-weight: 500;
		color: var(--color-white);
	}

	.pr-table__date {
		font-variant-numeric: tabular-nums;
		font-size: 0.75rem;
		color: var(--color-grey-400);
		white-space: nowrap;
	}

	.pr-table__actions-h,
	.pr-table__actions {
		width: 3rem;
		text-align: right;
	}

	.pr-icon-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		color: var(--color-grey-300);
		text-decoration: none;
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			color 150ms var(--ease-out);
	}

	.pr-icon-btn:hover {
		background-color: rgba(15, 164, 175, 0.12);
		border-color: rgba(15, 164, 175, 0.3);
		color: var(--color-teal-light);
	}

	/* ── Status pills ───────────────────── */
	.pr-badge {
		display: inline-flex;
		align-items: center;
		font-size: 0.6875rem;
		font-weight: 600;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.pr-badge--draft {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	.pr-badge--published {
		background-color: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}

	.pr-badge--archived {
		background-color: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
	}

	/* ── Pagination ─────────────────────── */
	.pr-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 0.5rem;
	}

	.pr-pag-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		min-height: 2.25rem;
		padding: 0.45rem 0.75rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: 0.875rem;
		font-weight: 600;
		font-family: var(--font-ui);
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out);
	}

	.pr-pag-btn:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.pr-pag-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.pr-pagination__info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
	}

	/* ── Create form ────────────────────── */
	.pr-form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.pr-form__grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 0.75rem;
	}

	.pr-form__actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.pr-form__error {
		padding: 0.65rem 0.85rem;
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: var(--radius-2xl);
		color: #fca5a5;
		font-size: 0.875rem;
		margin: 0;
	}

	.pr-form input,
	.pr-form select,
	.pr-form textarea {
		min-height: 3rem;
		padding: 0 1.25rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: 0.875rem;
		font-family: var(--font-ui);
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.pr-form input::placeholder,
	.pr-form textarea::placeholder {
		color: var(--color-grey-500);
	}

	.pr-form input:focus,
	.pr-form select:focus,
	.pr-form textarea:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.pr-form textarea {
		min-height: 5rem;
		resize: vertical;
	}

	/* ── Tablet 480px+ ──────────────────── */
	@media (min-width: 480px) {
		.pr-form__grid {
			grid-template-columns: 1fr 1fr;
		}
	}

	/* ── Tablet 768px+ ──────────────────── */
	@media (min-width: 768px) {
		.pr-admin {
			gap: 1.5rem;
		}

		.pr-admin__header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
		}


		.pr-filters {
			flex-direction: row;
			align-items: flex-end;
			padding: 1.5rem;
		}

		.pr-filters__field--search {
			flex: 1 1 18rem;
		}

		.pr-filters__field {
			flex: 0 0 11rem;
		}
	}
</style>
