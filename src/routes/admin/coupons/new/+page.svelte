<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, ApiError } from '$lib/api/client';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import Ticket from 'phosphor-svelte/lib/Ticket';
	import ArrowsClockwise from 'phosphor-svelte/lib/ArrowsClockwise';

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
		for (let i = 0; i < 8; i++) {
			result += chars[Math.floor(Math.random() * chars.length)];
		}
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

<svelte:head>
	<title>New Coupon - Admin - Explosive Swings</title>
</svelte:head>

<div class="cpn-new">
	<a href="/admin/coupons" class="cpn-new__back">
		<ArrowLeft size={18} />
		Back to Coupons
	</a>

	<div class="cpn-new__header">
		<Ticket size={24} weight="bold" />
		<h1 class="cpn-new__title">Create New Coupon</h1>
	</div>

	{#if error}
		<div class="cpn-new__error">{error}</div>
	{/if}

	<form onsubmit={handleSubmit} class="cpn-new__form">
		<div class="cpn-new__columns">
			<!-- Left Column -->
			<div class="cpn-new__col">
				<div class="cpn-new__field">
					<label for="code" class="cpn-new__label">Coupon Code</label>
					<div class="cpn-new__code-row">
						<input
							id="code"
							type="text"
							bind:value={code}
							required
							class="cpn-new__input"
							placeholder="e.g. SUMMER2026"
						/>
						<button type="button" onclick={generateCode} class="cpn-new__gen-btn" title="Generate random code">
							<ArrowsClockwise size={16} weight="bold" />
							Generate
						</button>
					</div>
				</div>

				<div class="cpn-new__field">
					<label for="description" class="cpn-new__label">Description</label>
					<textarea
						id="description"
						bind:value={description}
						class="cpn-new__textarea"
						rows="3"
						placeholder="Internal note about this coupon..."
					></textarea>
				</div>

				<div class="cpn-new__field">
					<label for="discountType" class="cpn-new__label">Discount Type</label>
					<select id="discountType" bind:value={discountType} class="cpn-new__input">
						<option value="percentage">Percentage</option>
						<option value="fixed">Fixed Amount</option>
						<option value="free_trial">Free Trial</option>
					</select>
				</div>

				<div class="cpn-new__field">
					<label for="value" class="cpn-new__label">
						Value {discountType === 'percentage' ? '(%)' : discountType === 'fixed' ? '($)' : '(days)'}
					</label>
					<input
						id="value"
						type="number"
						step="any"
						min="0"
						bind:value={value}
						required
						class="cpn-new__input"
						placeholder={discountType === 'percentage' ? '10' : discountType === 'fixed' ? '5.00' : '14'}
					/>
				</div>

				<div class="cpn-new__field">
					<label for="minPurchase" class="cpn-new__label">Min Purchase ($)</label>
					<input
						id="minPurchase"
						type="number"
						step="0.01"
						min="0"
						bind:value={minPurchase}
						class="cpn-new__input"
						placeholder="0.00"
					/>
				</div>

				<div class="cpn-new__field">
					<label for="maxDiscount" class="cpn-new__label">Max Discount ($)</label>
					<input
						id="maxDiscount"
						type="number"
						step="0.01"
						min="0"
						bind:value={maxDiscount}
						class="cpn-new__input"
						placeholder="No limit"
					/>
				</div>
			</div>

			<!-- Right Column -->
			<div class="cpn-new__col">
				<div class="cpn-new__field">
					<label for="usageLimit" class="cpn-new__label">Usage Limit</label>
					<input
						id="usageLimit"
						type="number"
						min="0"
						bind:value={usageLimit}
						class="cpn-new__input"
						placeholder="Unlimited"
					/>
				</div>

				<div class="cpn-new__field">
					<label for="perUserLimit" class="cpn-new__label">Per-User Limit</label>
					<input
						id="perUserLimit"
						type="number"
						min="0"
						bind:value={perUserLimit}
						class="cpn-new__input"
						placeholder="Unlimited"
					/>
				</div>

				<div class="cpn-new__field">
					<label for="startDate" class="cpn-new__label">Start Date</label>
					<input id="startDate" type="date" bind:value={startDate} class="cpn-new__input" />
				</div>

				<div class="cpn-new__field">
					<label for="expiryDate" class="cpn-new__label">Expiry Date</label>
					<input id="expiryDate" type="date" bind:value={expiryDate} class="cpn-new__input" />
				</div>

				<label class="cpn-new__toggle">
					<input type="checkbox" bind:checked={stackable} />
					<span class="cpn-new__toggle-track"><span class="cpn-new__toggle-thumb"></span></span>
					<span>Stackable</span>
				</label>

				<label class="cpn-new__toggle">
					<input type="checkbox" bind:checked={firstPurchaseOnly} />
					<span class="cpn-new__toggle-track"><span class="cpn-new__toggle-thumb"></span></span>
					<span>First Purchase Only</span>
				</label>

				<label class="cpn-new__toggle">
					<input type="checkbox" bind:checked={active} />
					<span class="cpn-new__toggle-track"><span class="cpn-new__toggle-thumb"></span></span>
					<span>Active</span>
				</label>
			</div>
		</div>

		<div class="cpn-new__actions">
			<a href="/admin/coupons" class="cpn-new__cancel">Cancel</a>
			<button type="submit" disabled={saving} class="cpn-new__submit">
				{saving ? 'Creating...' : 'Create Coupon'}
			</button>
		</div>
	</form>
</div>

<style>
	.cpn-new__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}
	.cpn-new__back:hover {
		color: var(--color-white);
	}
	.cpn-new__header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-teal);
		margin-bottom: 1.75rem;
	}
	.cpn-new__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.cpn-new__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
	}
	.cpn-new__form {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 2rem;
	}
	.cpn-new__columns {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 2rem;
		margin-bottom: 2rem;
	}
	.cpn-new__col {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.cpn-new__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.cpn-new__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}
	.cpn-new__input,
	.cpn-new__textarea {
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
	.cpn-new__input:focus,
	.cpn-new__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.cpn-new__input::placeholder,
	.cpn-new__textarea::placeholder {
		color: var(--color-grey-500);
	}
	.cpn-new__textarea {
		resize: vertical;
	}
	.cpn-new__code-row {
		display: flex;
		gap: 0.5rem;
	}
	.cpn-new__code-row .cpn-new__input {
		flex: 1;
	}
	.cpn-new__gen-btn {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 1rem;
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-lg);
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		white-space: nowrap;
		transition: background-color 200ms var(--ease-out);
	}
	.cpn-new__gen-btn:hover {
		background-color: rgba(15, 164, 175, 0.2);
	}
	/* Toggle switch */
	.cpn-new__toggle {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		cursor: pointer;
		padding: 0.4rem 0;
	}
	.cpn-new__toggle input[type='checkbox'] {
		position: absolute;
		opacity: 0;
		width: 0;
		height: 0;
	}
	.cpn-new__toggle-track {
		position: relative;
		width: 2.5rem;
		height: 1.35rem;
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		flex-shrink: 0;
		transition: background-color 200ms var(--ease-out);
	}
	.cpn-new__toggle input:checked + .cpn-new__toggle-track {
		background-color: var(--color-teal);
	}
	.cpn-new__toggle-thumb {
		position: absolute;
		top: 0.15rem;
		left: 0.15rem;
		width: 1.05rem;
		height: 1.05rem;
		background-color: var(--color-white);
		border-radius: var(--radius-full);
		transition: transform 200ms var(--ease-out);
	}
	.cpn-new__toggle input:checked + .cpn-new__toggle-track .cpn-new__toggle-thumb {
		transform: translateX(1.15rem);
	}
	.cpn-new__actions {
		display: flex;
		gap: 1rem;
		justify-content: flex-end;
		padding-top: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.cpn-new__cancel {
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
	.cpn-new__cancel:hover {
		border-color: rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}
	.cpn-new__submit {
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.cpn-new__submit:hover:not(:disabled) {
		opacity: 0.9;
	}
	.cpn-new__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 768px) {
		.cpn-new__columns {
			grid-template-columns: 1fr;
			gap: 1.25rem;
		}
	}
</style>
