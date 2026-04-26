<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import TicketIcon from 'phosphor-svelte/lib/TicketIcon';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

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
		coupon && coupon.usage_limit
			? Math.min(100, Math.round((coupon.usage_count / coupon.usage_limit) * 100))
			: 0
	);

	onMount(async () => {
		try {
			const data = await api.get<Coupon>(`/api/admin/coupons/${page.params.id}`);
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
		const ok = await confirmDialog({
			title: 'Delete this coupon?',
			message:
				'Existing redemptions are preserved for reporting, but the coupon will no longer be available.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		deleting = true;
		try {
			await api.del(`/api/admin/coupons/${page.params.id}`);
			goto('/admin/coupons');
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to delete';
			deleting = false;
		}
	}
</script>

<svelte:head>
	<title
		>{coupon ? `Edit ${coupon.code}` : 'Edit Coupon'} - Admin - Precision Options Signals</title
	>
</svelte:head>

<div class="ce">
	<a href="/admin/coupons" class="ce__back"><ArrowLeftIcon size={18} /> Back to Coupons</a>

	{#if loading}
		<p class="ce__status">Loading...</p>
	{:else if error && !coupon}
		<p class="ce__status">{error}</p>
	{:else if coupon}
		<div class="ce__hdr">
			<TicketIcon size={24} weight="bold" />
			<h1 class="ce__title">Edit Coupon</h1>
			<span class="ce__code">{coupon.code}</span>
		</div>

		{#if error}<div class="ce__alert ce__alert--err">{error}</div>{/if}
		{#if successMsg}<div class="ce__alert ce__alert--ok">{successMsg}</div>{/if}

		<div class="ce__stats">
			<div class="ce__sh">
				<UsersIcon size={18} />
				<h2 class="ce__st">Usage Statistics</h2>
			</div>
			<div class="ce__sbar">
				<div class="ce__snums">
					<span>{coupon.usage_count} used</span>
					<span
						>{coupon.usage_limit != null
							? `of ${coupon.usage_limit}`
							: 'Unlimited'}</span
					>
				</div>
				{#if coupon.usage_limit != null}
					<div class="ce__barbg">
						<div class="ce__barfill" style="width: {usagePercent}%"></div>
					</div>
				{/if}
			</div>
			{#if coupon.recent_usages && coupon.recent_usages.length > 0}
				<div class="ce__usages">
					<h3 class="ce__ulbl">Recent Usages</h3>
					<div class="ce__ulist">
						{#each coupon.recent_usages as usage (usage.user_email + usage.used_at)}
							<div class="ce__urow">
								<span class="ce__uemail">{usage.user_email}</span>
								<span class="ce__uamt">${usage.amount.toFixed(2)}</span>
								<span class="ce__udate"
									>{new Date(usage.used_at).toLocaleDateString()}</span
								>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>

		<form onsubmit={handleUpdate} class="ce__form">
			<div class="ce__cols">
				<div class="ce__col">
					<div class="ce__f">
						<label for="code" class="ce__lbl">Coupon Code</label>
						<input
							id="code"
							type="text"
							value={coupon.code}
							readonly
							class="ce__inp ce__inp--ro"
						/>
					</div>
					<div class="ce__f">
						<label for="description" class="ce__lbl">Description</label>
						<textarea
							id="description"
							bind:value={description}
							class="ce__ta"
							rows="3"
							placeholder="Internal note about this coupon..."
						></textarea>
					</div>
					<div class="ce__f">
						<label for="discountType" class="ce__lbl">Discount Type</label>
						<select id="discountType" bind:value={discountType} class="ce__inp">
							<option value="percentage">Percentage</option>
							<option value="fixed">Fixed Amount</option>
							<option value="free_trial">Free Trial</option>
						</select>
					</div>
					<div class="ce__f">
						<label for="value" class="ce__lbl"
							>Value {discountType === 'percentage'
								? '(%)'
								: discountType === 'fixed'
									? '($)'
									: '(days)'}</label
						>
						<input
							id="value"
							type="number"
							step="any"
							min="0"
							bind:value
							required
							class="ce__inp"
						/>
					</div>
					<div class="ce__f">
						<label for="minPurchase" class="ce__lbl">Min Purchase ($)</label>
						<input
							id="minPurchase"
							type="number"
							step="0.01"
							min="0"
							bind:value={minPurchase}
							class="ce__inp"
							placeholder="0.00"
						/>
					</div>
					<div class="ce__f">
						<label for="maxDiscount" class="ce__lbl">Max Discount ($)</label>
						<input
							id="maxDiscount"
							type="number"
							step="0.01"
							min="0"
							bind:value={maxDiscount}
							class="ce__inp"
							placeholder="No limit"
						/>
					</div>
				</div>
				<div class="ce__col">
					<div class="ce__f">
						<label for="usageLimit" class="ce__lbl">Usage Limit</label>
						<input
							id="usageLimit"
							type="number"
							min="0"
							bind:value={usageLimit}
							class="ce__inp"
							placeholder="Unlimited"
						/>
					</div>
					<div class="ce__f">
						<label for="perUserLimit" class="ce__lbl">Per-User Limit</label>
						<input
							id="perUserLimit"
							type="number"
							min="0"
							bind:value={perUserLimit}
							class="ce__inp"
							placeholder="Unlimited"
						/>
					</div>
					<div class="ce__f">
						<label for="startDate" class="ce__lbl">Start Date</label>
						<input id="startDate" type="date" bind:value={startDate} class="ce__inp" />
					</div>
					<div class="ce__f">
						<label for="expiryDate" class="ce__lbl">Expiry Date</label>
						<input
							id="expiryDate"
							type="date"
							bind:value={expiryDate}
							class="ce__inp"
						/>
					</div>
					<label class="ce__tog"
						><input type="checkbox" bind:checked={stackable} /><span class="ce__track"
							><span class="ce__thumb"></span></span
						><span>Stackable</span></label
					>
					<label class="ce__tog"
						><input type="checkbox" bind:checked={firstPurchaseOnly} /><span
							class="ce__track"><span class="ce__thumb"></span></span
						><span>First Purchase Only</span></label
					>
					<label class="ce__tog"
						><input type="checkbox" bind:checked={active} /><span class="ce__track"
							><span class="ce__thumb"></span></span
						><span>Active</span></label
					>
				</div>
			</div>
			<div class="ce__acts">
				<button type="button" onclick={handleDelete} disabled={deleting} class="ce__del"
					><TrashIcon size={16} weight="bold" />
					{deleting ? 'Deleting...' : 'Delete'}</button
				>
				<div class="ce__ar">
					<a href="/admin/coupons" class="ce__cancel">Cancel</a>
					<button type="submit" disabled={saving} class="ce__submit"
						><FloppyDiskIcon size={16} weight="bold" />
						{saving ? 'Saving...' : 'Update Coupon'}</button
					>
				</div>
			</div>
		</form>
	{/if}
</div>

<style>
	.ce__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}
	.ce__back:hover {
		color: var(--color-white);
	}
	.ce__status {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400);
	}
	.ce__hdr {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-teal);
		margin-bottom: 1.5rem;
	}
	.ce__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.ce__code {
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
	.ce__alert {
		padding: 0.75rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}
	.ce__alert--err {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}
	.ce__alert--ok {
		background-color: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.3);
		color: #86efac;
	}
	.ce__stats {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}
	.ce__sh {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-300);
		margin-bottom: 1rem;
	}
	.ce__st {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.ce__snums {
		display: flex;
		justify-content: space-between;
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin-bottom: 0.5rem;
	}
	.ce__barbg {
		width: 100%;
		height: 0.5rem;
		background-color: rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-full);
		overflow: hidden;
	}
	.ce__barfill {
		height: 100%;
		background: linear-gradient(90deg, var(--color-teal), var(--color-teal-light));
		border-radius: var(--radius-full);
		transition: width 300ms var(--ease-out);
	}
	.ce__usages {
		margin-top: 1.25rem;
	}
	.ce__ulbl {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.75rem;
	}
	.ce__ulist {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.ce__urow {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.5rem 0.75rem;
		background-color: rgba(255, 255, 255, 0.03);
		border-radius: var(--radius-md);
		font-size: var(--fs-sm);
	}
	.ce__uemail {
		flex: 1;
		color: var(--color-grey-300);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.ce__uamt {
		color: var(--color-green);
		font-weight: var(--w-semibold);
	}
	.ce__udate {
		color: var(--color-grey-500);
	}
	.ce__form {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 2rem;
	}
	.ce__cols {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 2rem;
		margin-bottom: 2rem;
	}
	.ce__col {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.ce__f {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.ce__lbl {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}
	.ce__inp,
	.ce__ta {
		width: 100%;
		padding: 0.65rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
		transition: border-color 200ms var(--ease-out);
	}
	.ce__inp:focus,
	.ce__ta:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.ce__inp::placeholder,
	.ce__ta::placeholder {
		color: var(--color-grey-500);
	}
	.ce__ta {
		resize: vertical;
	}
	.ce__inp--ro {
		opacity: 0.6;
		cursor: not-allowed;
		border-style: dashed;
	}
	.ce__tog {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		cursor: pointer;
		padding: 0.4rem 0;
	}
	.ce__tog input[type='checkbox'] {
		position: absolute;
		opacity: 0;
		width: 0;
		height: 0;
	}
	.ce__track {
		position: relative;
		width: 2.5rem;
		height: 1.35rem;
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		flex-shrink: 0;
		transition: background-color 200ms var(--ease-out);
	}
	.ce__tog input:checked + .ce__track {
		background-color: var(--color-teal);
	}
	.ce__thumb {
		position: absolute;
		top: 0.15rem;
		left: 0.15rem;
		width: 1.05rem;
		height: 1.05rem;
		background-color: var(--color-white);
		border-radius: var(--radius-full);
		transition: transform 200ms var(--ease-out);
	}
	.ce__tog input:checked + .ce__track .ce__thumb {
		transform: translateX(1.15rem);
	}
	.ce__acts {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-top: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.ce__ar {
		display: flex;
		gap: 1rem;
		align-items: center;
	}
	.ce__del {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.25rem;
		background-color: rgba(239, 68, 68, 0.08);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: var(--radius-2xl);
		color: var(--color-red);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}
	.ce__del:hover:not(:disabled) {
		background-color: rgba(239, 68, 68, 0.18);
	}
	.ce__del:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.ce__cancel {
		padding: 0.6rem 1.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition:
			border-color 200ms var(--ease-out),
			color 200ms var(--ease-out);
	}
	.ce__cancel:hover {
		border-color: rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}
	.ce__submit {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-2xl);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.ce__submit:hover:not(:disabled) {
		opacity: 0.9;
	}
	.ce__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	@media (max-width: 768px) {
		.ce__cols {
			grid-template-columns: 1fr;
			gap: 1.25rem;
		}
	}
</style>
