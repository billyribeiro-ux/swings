<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, ApiError } from '$lib/api/client';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import TicketIcon from 'phosphor-svelte/lib/TicketIcon';
	import ArrowsClockwiseIcon from 'phosphor-svelte/lib/ArrowsClockwiseIcon';

	let code = $state('');
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
	let saving = $state(false);
	let error = $state('');

	function generateCode() {
		const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
		let result = '';
		for (let i = 0; i < 8; i++) result += chars[Math.floor(Math.random() * chars.length)];
		code = result;
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		saving = true;
		error = '';
		try {
			await api.post('/api/admin/coupons', {
				code,
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
			goto('/admin/coupons');
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to create coupon';
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head><title>New Coupon - Admin - Precision Options Signals</title></svelte:head>

<div class="pg">
	<a href="/admin/coupons" class="back"><ArrowLeftIcon size={18} /> Back to Coupons</a>
	<div class="hdr">
		<TicketIcon size={24} weight="bold" />
		<h1>Create New Coupon</h1>
	</div>

	{#if error}<div class="err">{error}</div>{/if}

	<form onsubmit={handleSubmit} class="card">
		<div class="cols">
			<div class="col">
				<div class="fld">
					<label for="code">Coupon Code</label>
					<div class="code-row">
						<input
							id="code"
							type="text"
							bind:value={code}
							required
							placeholder="e.g. SUMMER2026"
						/>
						<button
							type="button"
							onclick={generateCode}
							class="gen"
							title="Generate random code"
						>
							<ArrowsClockwiseIcon size={16} weight="bold" /> Generate
						</button>
					</div>
				</div>
				<div class="fld">
					<label for="description">Description</label>
					<textarea
						id="description"
						bind:value={description}
						rows="3"
						placeholder="Internal note about this coupon..."
					></textarea>
				</div>
				<div class="fld">
					<label for="discountType">Discount Type</label>
					<select id="discountType" bind:value={discountType}>
						<option value="percentage">Percentage</option>
						<option value="fixed">Fixed Amount</option>
						<option value="free_trial">Free Trial</option>
					</select>
				</div>
				<div class="fld">
					<label for="value"
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
						placeholder={discountType === 'percentage'
							? '10'
							: discountType === 'fixed'
								? '5.00'
								: '14'}
					/>
				</div>
				<div class="fld">
					<label for="minPurchase">Min Purchase ($)</label>
					<input
						id="minPurchase"
						type="number"
						step="0.01"
						min="0"
						bind:value={minPurchase}
						placeholder="0.00"
					/>
				</div>
				<div class="fld">
					<label for="maxDiscount">Max Discount ($)</label>
					<input
						id="maxDiscount"
						type="number"
						step="0.01"
						min="0"
						bind:value={maxDiscount}
						placeholder="No limit"
					/>
				</div>
			</div>

			<div class="col">
				<div class="fld">
					<label for="usageLimit">Usage Limit</label>
					<input
						id="usageLimit"
						type="number"
						min="0"
						bind:value={usageLimit}
						placeholder="Unlimited"
					/>
				</div>
				<div class="fld">
					<label for="perUserLimit">Per-User Limit</label>
					<input
						id="perUserLimit"
						type="number"
						min="0"
						bind:value={perUserLimit}
						placeholder="Unlimited"
					/>
				</div>
				<div class="fld">
					<label for="startDate">Start Date</label>
					<input id="startDate" type="date" bind:value={startDate} />
				</div>
				<div class="fld">
					<label for="expiryDate">Expiry Date</label>
					<input id="expiryDate" type="date" bind:value={expiryDate} />
				</div>
				<label class="tog"
					><input type="checkbox" bind:checked={stackable} /><span class="track"
						><span class="thumb"></span></span
					><span>Stackable</span></label
				>
				<label class="tog"
					><input type="checkbox" bind:checked={firstPurchaseOnly} /><span class="track"
						><span class="thumb"></span></span
					><span>First Purchase Only</span></label
				>
				<label class="tog"
					><input type="checkbox" bind:checked={active} /><span class="track"
						><span class="thumb"></span></span
					><span>Active</span></label
				>
			</div>
		</div>

		<div class="actions">
			<a href="/admin/coupons" class="cancel">Cancel</a>
			<button type="submit" disabled={saving} class="submit"
				>{saving ? 'Creating...' : 'Create Coupon'}</button
			>
		</div>
	</form>
</div>

<style>
	.back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}
	.back:hover {
		color: var(--color-white);
	}
	.hdr {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-teal);
		margin-bottom: 1.75rem;
	}
	.hdr h1 {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.err {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
	}
	.card {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 2rem;
	}
	.cols {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 2rem;
		margin-bottom: 2rem;
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.fld {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.fld label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}
	input,
	textarea,
	select {
		width: 100%;
		padding: 0.65rem 0.85rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
		transition: border-color 200ms var(--ease-out);
	}
	input:focus,
	textarea:focus,
	select:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	input::placeholder,
	textarea::placeholder {
		color: var(--color-grey-500);
	}
	textarea {
		resize: vertical;
	}
	.code-row {
		display: flex;
		gap: 0.5rem;
	}
	.code-row input {
		flex: 1;
	}
	.gen {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 1rem;
		background: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-2xl);
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		white-space: nowrap;
		transition: background 200ms var(--ease-out);
	}
	.gen:hover {
		background: rgba(15, 164, 175, 0.2);
	}
	.tog {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		cursor: pointer;
		padding: 0.4rem 0;
	}
	.tog input[type='checkbox'] {
		position: absolute;
		opacity: 0;
		width: 0;
		height: 0;
	}
	.track {
		position: relative;
		width: 2.5rem;
		height: 1.35rem;
		background: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		flex-shrink: 0;
		transition: background 200ms var(--ease-out);
	}
	.tog input:checked + .track {
		background: var(--color-teal);
	}
	.thumb {
		position: absolute;
		top: 0.15rem;
		left: 0.15rem;
		width: 1.05rem;
		height: 1.05rem;
		background: var(--color-white);
		border-radius: var(--radius-full);
		transition: transform 200ms var(--ease-out);
	}
	.tog input:checked + .track .thumb {
		transform: translateX(1.15rem);
	}
	.actions {
		display: flex;
		gap: 1rem;
		justify-content: flex-end;
		padding-top: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.cancel {
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
	.cancel:hover {
		border-color: rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}
	.submit {
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-2xl);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.submit:hover:not(:disabled) {
		opacity: 0.9;
	}
	.submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	@media (max-width: 768px) {
		.cols {
			grid-template-columns: 1fr;
			gap: 1.25rem;
		}
	}
</style>
