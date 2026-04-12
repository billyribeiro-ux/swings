<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import FloppyDisk from 'phosphor-svelte/lib/FloppyDisk';
	import Trash from 'phosphor-svelte/lib/Trash';
	import Ticket from 'phosphor-svelte/lib/Ticket';
	import Users from 'phosphor-svelte/lib/Users';

	interface Coupon {
		id: string;
		code: string;
		description: string | null;
		discount_type: 'percentage' | 'fixed' | 'free_trial';
		value: number;
		min_purchase: number | null;
		max_discount: number | null;
		usage_limit: number | null;
		usage_count: number;
		per_user_limit: number | null;
		start_date: string | null;
		expiry_date: string | null;
		stackable: boolean;
		first_purchase_only: boolean;
		active: boolean;
		recent_usages?: { user_email: string; used_at: string; amount: number }[];
	}

	let coupon = $state<Coupon | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let deleting = $state(false);
	let error = $state('');
	let successMsg = $state('');

	let description = $state('');
	let discountType = $state<'percentage' | 'fixed' | 'free_trial'>('percentage');
	let value = $state('');
	let minPurchase = $state('');
	let maxDiscount = $state('');
	let usageLimit = $state('');
	let perUserLimit = $state('');
	let startDate = $state('');
	let expiryDate = $state('');
	let stackable = $state(false);
	let firstPurchaseOnly = $state(false);
	let active = $state(true);

	let usagePercent = $derived(
		coupon && coupon.usage_limit ? Math.min(100, Math.round((coupon.usage_count / coupon.usage_limit) * 100)) : 0
	);

	onMount(async () => {
		try {
			const id = page.params.id;
			const data = await api.get<Coupon>(`/api/admin/coupons/${id}`);
			coupon = data;
			description = data.description ?? '';
			discountType = data.discount_type;
			value = String(data.value);
			minPurchase = data.min_purchase != null ? String(data.min_purchase) : '';
			maxDiscount = data.max_discount != null ? String(data.max_discount) : '';
			usageLimit = data.usage_limit != null ? String(data.usage_limit) : '';
			perUserLimit = data.per_user_limit != null ? String(data.per_user_limit) : '';
			startDate = data.start_date ?? '';
			expiryDate = data.expiry_date ?? '';
			stackable = data.stackable;
			firstPurchaseOnly = data.first_purchase_only;
			active = data.active;
		} catch {
			error = 'Coupon not found';
		} finally {
			loading = false;
		}
	});

	async function handleUpdate(e: Event) {
		e.preventDefault();
		saving = true;
		error = '';
		successMsg = '';

		try {
			const updated = await api.put<Coupon>(`/api/admin/coupons/${page.params.id}`, {
				description: description || null,
				discount_type: discountType,
				value: value ? Number(value) : 0,
				min_purchase: minPurchase ? Number(minPurchase) : null,
				max_discount: maxDiscount ? Number(maxDiscount) : null,
				usage_limit: usageLimit ? Number(usageLimit) : null,
				per_user_limit: perUserLimit ? Number(perUserLimit) : null,
				start_date: startDate || null,
				expiry_date: expiryDate || null,
				stackable,
				first_purchase_only: firstPurchaseOnly,
				active
			});
			coupon = updated;
			successMsg = 'Coupon updated!';
			setTimeout(() => (successMsg = ''), 3000);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to update coupon';
		} finally {
			saving = false;
		}
	}

	async function handleDelete() {
		if (!confirm('Are you sure you want to delete this coupon? This cannot be undone.')) return;
		deleting = true;
		try {
			await api.del(`/api/admin/coupons/${page.params.id}`);
			goto('/admin/coupons');
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to delete coupon';
			deleting = false;
		}
	}
</script>

<svelte:head>
	<title>{coupon ? `Edit ${coupon.code}` : 'Edit Coupon'} - Admin - Explosive Swings</title>
</svelte:head>

<div class="cpn-edit">
	<a href="/admin/coupons" class="cpn-edit__back">
		<ArrowLeft size={18} />
		Back to Coupons
	</a>

	{#if loading}
		<p class="cpn-edit__status">Loading...</p>
	{:else if error && !coupon}
		<p class="cpn-edit__status">{error}</p>
	{:else if coupon}
		<div class="cpn-edit__header">
			<Ticket size={24} weight="bold" />
			<h1 class="cpn-edit__title">Edit Coupon</h1>
			<span class="cpn-edit__code-badge">{coupon.code}</span>
		</div>

		{#if error}
			<div class="cpn-edit__error">{error}</div>
		{/if}
		{#if successMsg}
			<div class="cpn-edit__success">{successMsg}</div>
		{/if}

		<!-- Usage Stats -->
		<div class="cpn-edit__stats">
			<div class="cpn-edit__stat-header">
				<Users size={18} />
				<h2 class="cpn-edit__stat-title">Usage Statistics</h2>
			</div>
			<div class="cpn-edit__stat-bar-wrap">
				<div class="cpn-edit__stat-nums">
					<span>{coupon.usage_count} used</span>
					<span>{coupon.usage_limit != null ? `of ${coupon.usage_limit}` : 'Unlimited'}</span>
				</div>
				{#if coupon.usage_limit != null}
					<div class="cpn-edit__bar-bg">
						<div class="cpn-edit__bar-fill" style="width: {usagePercent}%"></div>
					</div>
				{/if}
			</div>

			{#if coupon.recent_usages && coupon.recent_usages.length > 0}
				<div class="cpn-edit__usages">
					<h3 class="cpn-edit__usages-label">Recent Usages</h3>
					<div class="cpn-edit__usage-list">
						{#each coupon.recent_usages as usage}
							<div class="cpn-edit__usage-row">
								<span class="cpn-edit__usage-email">{usage.user_email}</span>
								<span class="cpn-edit__usage-amount">${usage.amount.toFixed(2)}</span>
								<span class="cpn-edit__usage-date">{new Date(usage.used_at).toLocaleDateString()}</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>

		<!-- Edit Form -->
		<form onsubmit={handleUpdate} class="cpn-edit__form">
			<div class="cpn-edit__columns">
				<!-- Left Column -->
				<div class="cpn-edit__col">
					<div class="cpn-edit__field">
						<label for="code" class="cpn-edit__label">Coupon Code</label>
						<input id="code" type="text" value={coupon.code} readonly class="cpn-edit__input cpn-edit__input--readonly" />
					</div>

					<div class="cpn-edit__field">
						<label for="description" class="cpn-edit__label">Description</label>
						<textarea
							id="description"
							bind:value={description}
							class="cpn-edit__textarea"
							rows="3"
							placeholder="Internal note about this coupon..."
						></textarea>
					</div>

					<div class="cpn-edit__field">
						<label for="discountType" class="cpn-edit__label">Discount Type</label>
						<select id="discountType" bind:value={discountType} class="cpn-edit__input">
							<option value="percentage">Percentage</option>
							<option value="fixed">Fixed Amount</option>
							<option value="free_trial">Free Trial</option>
						</select>
					</div>

					<div class="cpn-edit__field">
						<label for="value" class="cpn-edit__label">
							Value {discountType === 'percentage' ? '(%)' : discountType === 'fixed' ? '($)' : '(days)'}
						</label>
						<input id="value" type="number" step="any" min="0" bind:value={value} required class="cpn-edit__input" />
					</div>

					<div class="cpn-edit__field">
						<label for="minPurchase" class="cpn-edit__label">Min Purchase ($)</label>
						<input id="minPurchase" type="number" step="0.01" min="0" bind:value={minPurchase} class="cpn-edit__input" placeholder="0.00" />
					</div>

					<div class="cpn-edit__field">
						<label for="maxDiscount" class="cpn-edit__label">Max Discount ($)</label>
						<input id="maxDiscount" type="number" step="0.01" min="0" bind:value={maxDiscount} class="cpn-edit__input" placeholder="No limit" />
					</div>
				</div>

				<!-- Right Column -->
				<div class="cpn-edit__col">
					<div class="cpn-edit__field">
						<label for="usageLimit" class="cpn-edit__label">Usage Limit</label>
						<input id="usageLimit" type="number" min="0" bind:value={usageLimit} class="cpn-edit__input" placeholder="Unlimited" />
					</div>

					<div class="cpn-edit__field">
						<label for="perUserLimit" class="cpn-edit__label">Per-User Limit</label>
						<input id="perUserLimit" type="number" min="0" bind:value={perUserLimit} class="cpn-edit__input" placeholder="Unlimited" />
					</div>

					<div class="cpn-edit__field">
						<label for="startDate" class="cpn-edit__label">Start Date</label>
						<input id="startDate" type="date" bind:value={startDate} class="cpn-edit__input" />
					</div>

					<div class="cpn-edit__field">
						<label for="expiryDate" class="cpn-edit__label">Expiry Date</label>
						<input id="expiryDate" type="date" bind:value={expiryDate} class="cpn-edit__input" />
					</div>

					<label class="cpn-edit__toggle">
						<input type="checkbox" bind:checked={stackable} />
						<span class="cpn-edit__toggle-track"><span class="cpn-edit__toggle-thumb"></span></span>
						<span>Stackable</span>
					</label>

					<label class="cpn-edit__toggle">
						<input type="checkbox" bind:checked={firstPurchaseOnly} />
						<span class="cpn-edit__toggle-track"><span class="cpn-edit__toggle-thumb"></span></span>
						<span>First Purchase Only</span>
					</label>

					<label class="cpn-edit__toggle">
						<input type="checkbox" bind:checked={active} />
						<span class="cpn-edit__toggle-track"><span class="cpn-edit__toggle-thumb"></span></span>
						<span>Active</span>
					</label>
				</div>
			</div>

			<div class="cpn-edit__actions">
				<button type="button" onclick={handleDelete} disabled={deleting} class="cpn-edit__delete">
					<Trash size={16} weight="bold" />
					{deleting ? 'Deleting...' : 'Delete'}
				</button>
				<div class="cpn-edit__actions-right">
					<a href="/admin/coupons" class="cpn-edit__cancel">Cancel</a>
					<button type="submit" disabled={saving} class="cpn-edit__submit">
						<FloppyDisk size={16} weight="bold" />
						{saving ? 'Saving...' : 'Update Coupon'}
					</button>
				</div>
			</div>
		</form>
	{/if}
</div>

<style>
	.cpn-edit__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}
	.cpn-edit__back:hover { color: var(--color-white); }
	.cpn-edit__status {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400);
	}
	.cpn-edit__header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-teal);
		margin-bottom: 1.5rem;
	}
	.cpn-edit__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.cpn-edit__code-badge {
		margin-left: auto;
		padding: 0.35rem 0.85rem;
		background-color: rgba(15, 164, 175, 0.12);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-full);
		color: var(--color-teal-light);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		font-family: monospace;
		letter-spacing: 0.05em;
	}
	.cpn-edit__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}
	.cpn-edit__success {
		background-color: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.3);
		color: #86efac;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}
	/* Stats card */
	.cpn-edit__stats {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}
	.cpn-edit__stat-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-300);
		margin-bottom: 1rem;
	}
	.cpn-edit__stat-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.cpn-edit__stat-nums {
		display: flex;
		justify-content: space-between;
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin-bottom: 0.5rem;
	}
	.cpn-edit__bar-bg {
		width: 100%;
		height: 0.5rem;
		background-color: rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-full);
		overflow: hidden;
	}
	.cpn-edit__bar-fill {
		height: 100%;
		background: linear-gradient(90deg, var(--color-teal), var(--color-teal-light));
		border-radius: var(--radius-full);
		transition: width 300ms var(--ease-out);
	}
	.cpn-edit__usages { margin-top: 1.25rem; }
	.cpn-edit__usages-label {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.75rem;
	}
	.cpn-edit__usage-list {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.cpn-edit__usage-row {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.5rem 0.75rem;
		background-color: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius-md);
		font-size: var(--fs-sm);
	}
	.cpn-edit__usage-email {
		flex: 1;
		color: var(--color-grey-300);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.cpn-edit__usage-amount {
		color: var(--color-green);
		font-weight: var(--w-semibold);
	}
	.cpn-edit__usage-date { color: var(--color-grey-500); }
	/* Form */
	.cpn-edit__form {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 2rem;
	}
	.cpn-edit__columns {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 2rem;
		margin-bottom: 2rem;
	}
	.cpn-edit__col {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.cpn-edit__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.cpn-edit__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}
	.cpn-edit__input,
	.cpn-edit__textarea {
		width: 100%;
		padding: 0.65rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
		transition: border-color 200ms var(--ease-out);
	}
	.cpn-edit__input:focus,
	.cpn-edit__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.cpn-edit__input::placeholder,
	.cpn-edit__textarea::placeholder {
		color: var(--color-grey-500);
	}
	.cpn-edit__textarea { resize: vertical; }
	.cpn-edit__input--readonly {
		opacity: 0.6;
		cursor: not-allowed;
		border-style: dashed;
	}
	/* Toggle switch */
	.cpn-edit__toggle {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		cursor: pointer;
		padding: 0.4rem 0;
	}
	.cpn-edit__toggle input[type='checkbox'] {
		position: absolute;
		opacity: 0;
		width: 0;
		height: 0;
	}
	.cpn-edit__toggle-track {
		position: relative;
		width: 2.5rem;
		height: 1.35rem;
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		flex-shrink: 0;
		transition: background-color 200ms var(--ease-out);
	}
	.cpn-edit__toggle input:checked + .cpn-edit__toggle-track {
		background-color: var(--color-teal);
	}
	.cpn-edit__toggle-thumb {
		position: absolute;
		top: 0.15rem;
		left: 0.15rem;
		width: 1.05rem;
		height: 1.05rem;
		background-color: var(--color-white);
		border-radius: var(--radius-full);
		transition: transform 200ms var(--ease-out);
	}
	.cpn-edit__toggle input:checked + .cpn-edit__toggle-track .cpn-edit__toggle-thumb {
		transform: translateX(1.15rem);
	}
	/* Actions */
	.cpn-edit__actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-top: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.cpn-edit__actions-right {
		display: flex;
		gap: 1rem;
		align-items: center;
	}
	.cpn-edit__delete {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.25rem;
		background-color: rgba(239, 68, 68, 0.08);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: var(--radius-lg);
		color: var(--color-red);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}
	.cpn-edit__delete:hover:not(:disabled) {
		background-color: rgba(239, 68, 68, 0.18);
	}
	.cpn-edit__delete:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.cpn-edit__cancel {
		padding: 0.6rem 1.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition:
			border-color 200ms var(--ease-out),
			color 200ms var(--ease-out);
	}
	.cpn-edit__cancel:hover {
		border-color: rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}
	.cpn-edit__submit {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.cpn-edit__submit:hover:not(:disabled) { opacity: 0.9; }
	.cpn-edit__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 768px) {
		.cpn-edit__columns {
			grid-template-columns: 1fr;
			gap: 1.25rem;
		}
	}
</style>
