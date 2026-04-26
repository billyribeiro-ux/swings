<!--
  EC-01 admin product detail — edit product, manage variants, downloadable
  assets, and bundle items.

  PE7 primitives only (Button, FormField); no Tailwind, no Lucide.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button, FormField } from '$lib/components/shared';
	import { productsApi } from '$lib/api/products';
	import type {
		ProductDetail,
		ProductStatus,
		ProductVariant,
		DownloadableAsset,
		BundleItem,
		BundleItemInput,
		CreateVariantRequest,
		CreateAssetRequest
	} from '$lib/api/products';
	import { ApiError } from '$lib/api/client';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	const productId = $derived(page.params.id);

	let detail = $state<ProductDetail | null>(null);
	let loading = $state(true);
	let errorMessage = $state<string | null>(null);
	let saving = $state(false);

	// Editable product fields (loaded from `detail`).
	let name = $state('');
	let slug = $state('');
	let description = $state('');
	let priceDollars = $state('');
	let currency = $state('USD');
	let taxClass = $state('standard');
	let stripeProductId = $state('');
	let stripePriceId = $state('');
	let seoTitle = $state('');
	let seoDescription = $state('');

	// New-variant form.
	let vSku = $state('');
	let vName = $state('');
	let vPriceDollars = $state('');

	// New-asset form.
	let aStorageKey = $state('');
	let aFilename = $state('');
	let aMime = $state('application/zip');
	let aSize = $state(0);
	let aSha256 = $state('');
	let aAccess = $state<'purchase_required' | 'member_tier' | 'public'>('purchase_required');

	// Bundle editor — working copy, flushed on save.
	let bundleItems = $state<BundleItemInput[]>([]);

	async function load() {
		if (!productId) return;
		loading = true;
		errorMessage = null;
		try {
			detail = await productsApi.adminGet(productId);
			name = detail.name;
			slug = detail.slug;
			description = detail.description ?? '';
			priceDollars =
				detail.price_cents !== null && detail.price_cents !== undefined
					? (detail.price_cents / 100).toFixed(2)
					: '';
			currency = detail.currency;
			taxClass = detail.tax_class;
			stripeProductId = detail.stripe_product_id ?? '';
			stripePriceId = detail.stripe_price_id ?? '';
			seoTitle = detail.seo_title ?? '';
			seoDescription = detail.seo_description ?? '';
			bundleItems = detail.bundle_items.map((bi: BundleItem) => ({
				child_product_id: bi.child_product_id,
				child_variant_id: bi.child_variant_id,
				quantity: bi.quantity,
				position: bi.position
			}));
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Load failed';
		} finally {
			loading = false;
		}
	}

	// One-shot load on mount. `productId` is a route param resolved at mount,
	// so we don't need a reactive `$effect` (which previously risked
	// `effect_update_depth_exceeded` because `load()` mutates `detail`,
	// which is read elsewhere in the component).
	onMount(load);

	async function save(e: SubmitEvent) {
		e.preventDefault();
		if (saving || !detail) return;
		saving = true;
		errorMessage = null;
		try {
			const priceCents = priceDollars ? Math.round(Number(priceDollars) * 100) : null;
			await productsApi.adminUpdate(detail.id, {
				name,
				slug,
				description: description || null,
				price_cents: priceCents,
				currency,
				tax_class: taxClass,
				stripe_product_id: stripeProductId || null,
				stripe_price_id: stripePriceId || null,
				seo_title: seoTitle || null,
				seo_description: seoDescription || null
			});
			await load();
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Save failed';
		} finally {
			saving = false;
		}
	}

	async function setStatus(next: ProductStatus) {
		if (!detail) return;
		try {
			await productsApi.adminSetStatus(detail.id, next);
			await load();
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Status change failed';
		}
	}

	async function addVariant() {
		if (!detail) return;
		const body: CreateVariantRequest = {
			sku: vSku || null,
			name: vName || null,
			price_cents: vPriceDollars ? Math.round(Number(vPriceDollars) * 100) : null
		};
		try {
			await productsApi.adminAddVariant(detail.id, body);
			vSku = '';
			vName = '';
			vPriceDollars = '';
			await load();
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Add variant failed';
		}
	}

	async function deleteVariant(v: ProductVariant) {
		if (!detail) return;
		const ok = await confirmDialog({
			title: `Delete variant ${v.sku ?? v.id}?`,
			message:
				'The variant will be removed from this product. Existing orders that reference it are preserved.',
			confirmLabel: 'Delete variant',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await productsApi.adminDeleteVariant(detail.id, v.id);
			await load();
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Delete variant failed';
		}
	}

	async function addAsset() {
		if (!detail) return;
		const body: CreateAssetRequest = {
			storage_key: aStorageKey,
			filename: aFilename,
			mime_type: aMime,
			size_bytes: aSize,
			sha256: aSha256,
			access_policy: aAccess
		};
		try {
			await productsApi.adminAddAsset(detail.id, body);
			aStorageKey = '';
			aFilename = '';
			aSize = 0;
			aSha256 = '';
			await load();
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Add asset failed';
		}
	}

	async function deleteAsset(asset: DownloadableAsset) {
		if (!detail) return;
		const ok = await confirmDialog({
			title: `Delete asset "${asset.filename}"?`,
			message:
				'Customers who already purchased this product will lose download access to this asset.',
			confirmLabel: 'Delete asset',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await productsApi.adminDeleteAsset(detail.id, asset.id);
			await load();
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Delete asset failed';
		}
	}

	function addBundleRow() {
		bundleItems = [
			...bundleItems,
			{
				child_product_id: '',
				child_variant_id: null,
				quantity: 1,
				position: bundleItems.length
			}
		];
	}

	function removeBundleRow(idx: number) {
		bundleItems = bundleItems.filter((_, i) => i !== idx);
	}

	async function saveBundle() {
		if (!detail) return;
		try {
			await productsApi.adminSetBundleItems(detail.id, bundleItems);
			await load();
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Save bundle failed';
		}
	}

	async function deleteProduct() {
		if (!detail) return;
		const ok = await confirmDialog({
			title: `Permanently delete "${detail.name}"?`,
			message:
				'The product, its variants, downloadable assets, and bundle configuration will all be removed.',
			confirmLabel: 'Delete product',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await productsApi.adminDelete(detail.id);
			goto('/admin/products');
		} catch (err) {
			errorMessage = err instanceof ApiError ? err.message : 'Delete failed';
		}
	}
</script>

<svelte:head>
	<title>{detail?.name ?? 'Product'} - Admin</title>
</svelte:head>

<div class="pr-detail">
	<div class="pr-detail__breadcrumb">
		{#snippet backIcon()}<ArrowLeftIcon size={14} weight="bold" />{/snippet}
		<Button variant="ghost" size="sm" href="/admin/products" iconLeading={backIcon}>
			Back to products
		</Button>
	</div>

	{#if loading}
		<p class="pr-detail__loading">Loading…</p>
	{:else if !detail}
		<p class="pr-detail__error">Product not found.</p>
	{:else}
		<header class="pr-detail__header">
			<div>
				<h1>{detail.name}</h1>
				<p class="pr-detail__meta">
					<span class="pr-badge pr-badge--{detail.status}">{detail.status}</span>
					<span class="pr-detail__type">{detail.product_type}</span>
				</p>
			</div>
			<div class="pr-detail__header-actions">
				{#if detail.status !== 'published'}
					<Button variant="primary" onclick={() => setStatus('published')}>Publish</Button
					>
				{/if}
				{#if detail.status === 'published'}
					<Button variant="secondary" onclick={() => setStatus('draft')}>Unpublish</Button
					>
				{/if}
				{#if detail.status !== 'archived'}
					<Button variant="tertiary" onclick={() => setStatus('archived')}>Archive</Button
					>
				{/if}
				<Button variant="danger" onclick={deleteProduct}>Delete</Button>
			</div>
		</header>

		{#if errorMessage}
			<p class="pr-detail__error" role="alert">{errorMessage}</p>
		{/if}

		<section class="pr-panel">
			<h2>Details</h2>
			<form class="pr-form" onsubmit={save}>
				<FormField for="d-name" label="Name" required>
					{#snippet children({ describedBy, invalid, required })}
						<input
							id="d-name"
							type="text"
							bind:value={name}
							{required}
							aria-invalid={invalid}
							aria-describedby={describedBy}
						/>
					{/snippet}
				</FormField>
				<FormField for="d-slug" label="Slug" required>
					{#snippet children({ describedBy, invalid, required })}
						<input
							id="d-slug"
							type="text"
							pattern="[a-z0-9-]+"
							bind:value={slug}
							{required}
							aria-invalid={invalid}
							aria-describedby={describedBy}
						/>
					{/snippet}
				</FormField>
				<div class="pr-form__grid">
					<FormField for="d-price" label="Price (USD)">
						{#snippet children({ describedBy, invalid })}
							<input
								id="d-price"
								type="number"
								step="0.01"
								min="0"
								bind:value={priceDollars}
								aria-invalid={invalid}
								aria-describedby={describedBy}
							/>
						{/snippet}
					</FormField>
					<FormField for="d-currency" label="Currency">
						{#snippet children({ describedBy, invalid })}
							<input
								id="d-currency"
								type="text"
								maxlength="3"
								bind:value={currency}
								aria-invalid={invalid}
								aria-describedby={describedBy}
							/>
						{/snippet}
					</FormField>
					<FormField for="d-tax" label="Tax class">
						{#snippet children({ describedBy, invalid })}
							<input
								id="d-tax"
								type="text"
								bind:value={taxClass}
								aria-invalid={invalid}
								aria-describedby={describedBy}
							/>
						{/snippet}
					</FormField>
				</div>
				<FormField for="d-desc" label="Description">
					{#snippet children({ describedBy, invalid })}
						<textarea
							id="d-desc"
							rows={4}
							bind:value={description}
							aria-invalid={invalid}
							aria-describedby={describedBy}
						></textarea>
					{/snippet}
				</FormField>
				<div class="pr-form__grid">
					<FormField for="d-stripe-product" label="Stripe product id">
						{#snippet children({ describedBy, invalid })}
							<input
								id="d-stripe-product"
								type="text"
								bind:value={stripeProductId}
								aria-invalid={invalid}
								aria-describedby={describedBy}
							/>
						{/snippet}
					</FormField>
					<FormField for="d-stripe-price" label="Stripe price id">
						{#snippet children({ describedBy, invalid })}
							<input
								id="d-stripe-price"
								type="text"
								bind:value={stripePriceId}
								aria-invalid={invalid}
								aria-describedby={describedBy}
							/>
						{/snippet}
					</FormField>
				</div>
				<FormField for="d-seo-title" label="SEO title">
					{#snippet children({ describedBy, invalid })}
						<input
							id="d-seo-title"
							type="text"
							bind:value={seoTitle}
							aria-invalid={invalid}
							aria-describedby={describedBy}
						/>
					{/snippet}
				</FormField>
				<FormField for="d-seo-desc" label="SEO description">
					{#snippet children({ describedBy, invalid })}
						<textarea
							id="d-seo-desc"
							rows={2}
							bind:value={seoDescription}
							aria-invalid={invalid}
							aria-describedby={describedBy}
						></textarea>
					{/snippet}
				</FormField>
				<div class="pr-form__actions">
					<Button type="submit" loading={saving} disabled={saving}>Save changes</Button>
				</div>
			</form>
		</section>

		<section class="pr-panel">
			<h2>Variants</h2>
			{#if detail.variants.length === 0}
				<p class="pr-panel__empty">No variants.</p>
			{:else}
				<table class="pr-sub-table">
					<thead>
						<tr>
							<th>SKU</th>
							<th>Name</th>
							<th class="pr-table__right">Price</th>
							<th>Active</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each detail.variants as v (v.id)}
							<tr>
								<td class="pr-table__mono">{v.sku ?? '—'}</td>
								<td>{v.name ?? '—'}</td>
								<td class="pr-table__right pr-table__mono">
									{v.price_cents !== null && v.price_cents !== undefined
										? `$${(v.price_cents / 100).toFixed(2)}`
										: 'inherit'}
								</td>
								<td>{v.is_active ? 'Yes' : 'No'}</td>
								<td>
									{#snippet trashIcon()}<TrashIcon
											size={14}
											weight="bold"
										/>{/snippet}
									<Button
										variant="ghost"
										size="sm"
										onclick={() => deleteVariant(v)}
										iconLeading={trashIcon}
										aria-label="Delete variant"
									>
										Delete
									</Button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}

			<h3>Add variant</h3>
			<div class="pr-sub-form">
				<input
					id="v-sku"
					name="variant-sku"
					type="text"
					placeholder="SKU"
					aria-label="Variant SKU"
					bind:value={vSku}
				/>
				<input
					id="v-name"
					name="variant-name"
					type="text"
					placeholder="Name"
					aria-label="Variant name"
					bind:value={vName}
				/>
				<input
					id="v-price"
					name="variant-price"
					type="number"
					step="0.01"
					min="0"
					placeholder="Price override"
					aria-label="Variant price override"
					bind:value={vPriceDollars}
				/>
				<Button variant="secondary" onclick={addVariant}>Add</Button>
			</div>
		</section>

		<section class="pr-panel">
			<h2>Downloadable assets</h2>
			{#if detail.assets.length === 0}
				<p class="pr-panel__empty">No assets yet.</p>
			{:else}
				<table class="pr-sub-table">
					<thead>
						<tr>
							<th>Filename</th>
							<th>Storage key</th>
							<th class="pr-table__right">Size</th>
							<th>Access</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each detail.assets as a (a.id)}
							<tr>
								<td>{a.filename}</td>
								<td class="pr-table__mono">{a.storage_key}</td>
								<td class="pr-table__right pr-table__mono">
									{(a.size_bytes / (1024 * 1024)).toFixed(2)} MB
								</td>
								<td>{a.access_policy}</td>
								<td>
									{#snippet trashIcon2()}<TrashIcon
											size={14}
											weight="bold"
										/>{/snippet}
									<Button
										variant="ghost"
										size="sm"
										onclick={() => deleteAsset(a)}
										iconLeading={trashIcon2}
										aria-label="Delete asset"
									>
										Delete
									</Button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}

			<h3>Add asset</h3>
			<p class="pr-panel__note">
				EC-07 will wire signed-URL issuance against the storage_key; this panel records the
				metadata.
			</p>
			<div class="pr-sub-form pr-sub-form--asset">
				<input
					id="a-storage-key"
					name="asset-storage-key"
					type="text"
					placeholder="R2 storage key"
					aria-label="Asset R2 storage key"
					bind:value={aStorageKey}
				/>
				<input
					id="a-filename"
					name="asset-filename"
					type="text"
					placeholder="Filename"
					aria-label="Asset filename"
					bind:value={aFilename}
				/>
				<input
					id="a-mime"
					name="asset-mime-type"
					type="text"
					placeholder="MIME type"
					aria-label="Asset MIME type"
					bind:value={aMime}
				/>
				<input
					id="a-size"
					name="asset-size-bytes"
					type="number"
					min="0"
					placeholder="Size (bytes)"
					aria-label="Asset size in bytes"
					bind:value={aSize}
				/>
				<input
					id="a-sha256"
					name="asset-sha256"
					type="text"
					placeholder="SHA-256"
					aria-label="Asset SHA-256 checksum"
					bind:value={aSha256}
				/>
				<select
					id="a-access"
					name="asset-access-policy"
					aria-label="Asset access policy"
					bind:value={aAccess}
				>
					<option value="purchase_required">Purchase required</option>
					<option value="member_tier">Member tier</option>
					<option value="public">Public</option>
				</select>
				<Button variant="secondary" onclick={addAsset}>Add</Button>
			</div>
		</section>

		{#if detail.product_type === 'bundle'}
			<section class="pr-panel">
				<h2>Bundle items</h2>
				{#if bundleItems.length === 0}
					<p class="pr-panel__empty">No child products configured.</p>
				{:else}
					<table class="pr-sub-table">
						<thead>
							<tr>
								<th>Child product id</th>
								<th>Variant id (optional)</th>
								<th class="pr-table__right">Quantity</th>
								<th class="pr-table__right">Position</th>
								<th></th>
							</tr>
						</thead>
						<tbody>
							{#each bundleItems as item, idx (idx)}
								<tr>
									<td>
										<input
											type="text"
											class="pr-sub-input"
											aria-label={`Row ${idx + 1} child product id`}
											bind:value={item.child_product_id}
										/>
									</td>
									<td>
										<input
											type="text"
											class="pr-sub-input"
											aria-label={`Row ${idx + 1} child variant id`}
											value={item.child_variant_id ?? ''}
											oninput={(e) => {
												const v = (
													e.target as HTMLInputElement
												).value.trim();
												item.child_variant_id = v || null;
											}}
										/>
									</td>
									<td class="pr-table__right">
										<input
											type="number"
											class="pr-sub-input pr-sub-input--num"
											min="1"
											aria-label={`Row ${idx + 1} quantity`}
											bind:value={item.quantity}
										/>
									</td>
									<td class="pr-table__right">
										<input
											type="number"
											class="pr-sub-input pr-sub-input--num"
											min="0"
											aria-label={`Row ${idx + 1} position`}
											bind:value={item.position}
										/>
									</td>
									<td>
										<Button
											variant="ghost"
											size="sm"
											onclick={() => removeBundleRow(idx)}
											aria-label="Remove row"
										>
											Remove
										</Button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{/if}
				<div class="pr-bundle__actions">
					<Button variant="tertiary" onclick={addBundleRow}>Add row</Button>
					<Button onclick={saveBundle}>Save bundle</Button>
				</div>
			</section>
		{/if}
	{/if}
</div>

<style>
	.pr-detail {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.pr-detail__breadcrumb {
		margin-bottom: -0.5rem;
	}
	.pr-detail__header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
		flex-wrap: wrap;
	}
	.pr-detail__header h1 {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.pr-detail__meta {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		margin-top: 0.25rem;
	}
	.pr-detail__type {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: capitalize;
	}
	.pr-detail__header-actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.pr-detail__loading,
	.pr-detail__error {
		color: var(--color-grey-400);
	}
	.pr-detail__error {
		color: var(--status-danger-500, #ef4444);
	}
	.pr-panel {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.pr-panel h2 {
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}
	.pr-panel h3 {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-grey-300);
		margin-top: 0.5rem;
	}
	.pr-panel__empty {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.pr-panel__note {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
	}
	.pr-form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.pr-form__grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
		gap: 0.75rem;
	}
	.pr-form__actions {
		display: flex;
		justify-content: flex-end;
	}
	.pr-form input,
	.pr-form textarea {
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius-md);
		background: var(--color-navy-dark, #0b1220);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		font-size: var(--fs-sm);
	}
	.pr-sub-table {
		width: 100%;
		border-collapse: collapse;
	}
	.pr-sub-table th {
		text-align: left;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 0.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.pr-sub-table td {
		padding: 0.5rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.pr-table__mono {
		font-family: var(--font-mono, monospace);
		font-size: var(--fs-xs);
	}
	.pr-table__right {
		text-align: right;
	}
	.pr-sub-form {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
		gap: 0.5rem;
		align-items: center;
	}
	.pr-sub-form--asset {
		grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
	}
	.pr-sub-form input,
	.pr-sub-form select,
	.pr-sub-input {
		padding: 0.45rem 0.6rem;
		border-radius: var(--radius-md);
		background: var(--color-navy-dark, #0b1220);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		font-size: var(--fs-sm);
		width: 100%;
		box-sizing: border-box;
	}
	.pr-sub-input--num {
		text-align: right;
	}
	.pr-bundle__actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
		margin-top: 0.75rem;
	}
	.pr-badge {
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
</style>
