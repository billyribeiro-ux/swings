<!--
  EC-01 admin products — list + create.

  Uses PE7 shared primitives (Button, Dialog, FormField); no Tailwind, no
  Lucide. Icons come from `@iconify-json/ph` via phosphor-svelte (per PE7
  convention, same as every other admin surface in the repo).
-->
<script lang="ts">
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
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import PackageIcon from 'phosphor-svelte/lib/PackageIcon';

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
			console.error('[products] list failed', err);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		load();
	});

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
			createOpen = false;
			goto(`/admin/products/${created.id}`);
		} catch (err) {
			createError = err instanceof ApiError ? err.message : 'Create failed';
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
</script>

<svelte:head>
	<title>Products - Admin</title>
</svelte:head>

<div class="pr-admin">
	<header class="pr-admin__header">
		<div>
			<h1 class="pr-admin__title">Products</h1>
			<p class="pr-admin__count">
				{total} total · {publishedCount} published · {draftCount} draft
			</p>
		</div>
		{#snippet plusIcon()}<PlusIcon size={16} weight="bold" />{/snippet}
		<Button onclick={openCreate} iconLeading={plusIcon}>New Product</Button>
	</header>

	<div class="pr-filters">
		<input
			type="search"
			class="pr-search"
			placeholder="Search name or slug…"
			bind:value={search}
			onchange={() => {
				page = 1;
				load();
			}}
		/>
		<select
			class="pr-select"
			bind:value={statusFilter}
			onchange={() => {
				page = 1;
			}}
		>
			<option value="">All statuses</option>
			<option value="draft">Draft</option>
			<option value="published">Published</option>
			<option value="archived">Archived</option>
		</select>
		<select
			class="pr-select"
			bind:value={typeFilter}
			onchange={() => {
				page = 1;
			}}
		>
			<option value="">All types</option>
			<option value="simple">Simple</option>
			<option value="subscription">Subscription</option>
			<option value="downloadable">Downloadable</option>
			<option value="bundle">Bundle</option>
		</select>
	</div>

	{#if loading}
		<div class="pr-admin__loading">Loading…</div>
	{:else if rows.length === 0}
		<div class="pr-admin__empty">
			<PackageIcon size={32} weight="light" />
			<p>No products yet.</p>
			<Button variant="secondary" onclick={openCreate}>Create your first product</Button>
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
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each rows as product (product.id)}
						<tr>
							<td class="pr-table__name">{product.name}</td>
							<td class="pr-table__mono">{product.slug}</td>
							<td>{product.product_type}</td>
							<td><span class={statusClass(product.status)}>{product.status}</span></td>
							<td class="pr-table__right pr-table__mono">
								{fmtMoney(product.price_cents, product.currency)}
							</td>
							<td class="pr-table__right pr-table__mono">
								{new Date(product.updated_at).toLocaleDateString()}
							</td>
							<td>
								<Button
									variant="ghost"
									size="sm"
									href={`/admin/products/${product.id}`}
									aria-label={`Edit ${product.name}`}
								>
									Edit
								</Button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<nav class="pr-pagination" aria-label="Pagination">
				<Button
					variant="secondary"
					size="sm"
					disabled={page <= 1}
					onclick={() => {
						page = Math.max(1, page - 1);
					}}
				>
					Previous
				</Button>
				<span class="pr-pagination__info">Page {page} of {totalPages}</span>
				<Button
					variant="secondary"
					size="sm"
					disabled={page >= totalPages}
					onclick={() => {
						page = Math.min(totalPages, page + 1);
					}}
				>
					Next
				</Button>
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
		gap: 1rem;
	}
	.pr-admin__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
	}
	.pr-admin__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.pr-admin__count {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-top: 0.15rem;
	}
	.pr-filters {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.pr-search,
	.pr-select {
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius-md);
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		font-size: var(--fs-sm);
	}
	.pr-search {
		flex: 1 1 14rem;
	}
	.pr-admin__loading,
	.pr-admin__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
		padding: 2.5rem 1rem;
		background: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		color: var(--color-grey-400);
	}
	.pr-table-wrap {
		overflow-x: auto;
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}
	.pr-table {
		width: 100%;
		border-collapse: collapse;
	}
	.pr-table th {
		text-align: left;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.pr-table td {
		padding: 0.75rem 1rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.pr-table tbody tr:hover {
		background: rgba(255, 255, 255, 0.02);
	}
	.pr-table__name {
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}
	.pr-table__mono {
		font-family: var(--font-mono, monospace);
		font-size: var(--fs-xs);
	}
	.pr-table__right {
		text-align: right;
	}
	.pr-badge {
		display: inline-block;
		font-size: var(--fs-xs);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: capitalize;
	}
	.pr-badge--draft {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}
	.pr-badge--published {
		background: rgba(34, 197, 94, 0.12);
		color: #22c55e;
	}
	.pr-badge--archived {
		background: rgba(239, 68, 68, 0.12);
		color: #ef4444;
	}
	.pr-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1rem;
	}
	.pr-pagination__info {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}
	.pr-form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.pr-form__grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}
	.pr-form__actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}
	.pr-form__error {
		color: var(--status-danger-500, #ef4444);
		font-size: var(--fs-sm);
	}
	.pr-form input,
	.pr-form select,
	.pr-form textarea {
		padding: 0.55rem 0.75rem;
		border-radius: var(--radius-md);
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		font-size: var(--fs-sm);
	}
</style>
