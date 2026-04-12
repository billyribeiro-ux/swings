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
		id: string; code: string; description: string | null;
		discount_type: 'percentage' | 'fixed' | 'free_trial'; value: number;
		min_purchase: number | null; max_discount: number | null;
		usage_limit: number | null; usage_count: number; per_user_limit: number | null;
		start_date: string | null; expiry_date: string | null;
		stackable: boolean; first_purchase_only: boolean; active: boolean;
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
		} catch { error = 'Coupon not found'; } finally { loading = false; }
	});

	async function handleUpdate(e: Event) {
		e.preventDefault();
		saving = true; error = ''; successMsg = '';
		try {
			const updated = await api.put<Coupon>(`/api/admin/coupons/${page.params.id}`, {
				description: description || null, discount_type: discountType,
				value: value ? Number(value) : 0, min_purchase: minPurchase ? Number(minPurchase) : null,
				max_discount: maxDiscount ? Number(maxDiscount) : null,
				usage_limit: usageLimit ? Number(usageLimit) : null,
				per_user_limit: perUserLimit ? Number(perUserLimit) : null,
				start_date: startDate || null, expiry_date: expiryDate || null,
				stackable, first_purchase_only: firstPurchaseOnly, active
			});
			coupon = updated;
			successMsg = 'Coupon updated!';
			setTimeout(() => (successMsg = ''), 3000);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to update coupon';
		} finally { saving = false; }
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

<svelte:head><title>{coupon ? `Edit ${coupon.code}` : 'Edit Coupon'} - Admin - Explosive Swings</title></svelte:head>

<div class="pg">
	<a href="/admin/coupons" class="back"><ArrowLeft size={18} /> Back to Coupons</a>

	{#if loading}
		<p class="status">Loading...</p>
	{:else if error && !coupon}
		<p class="status">{error}</p>
	{:else if coupon}
		<div class="hdr">
			<Ticket size={24} weight="bold" /><h1>Edit Coupon</h1>
			<span class="badge">{coupon.code}</span>
		</div>

		{#if error}<div class="err">{error}</div>{/if}
		{#if successMsg}<div class="ok">{successMsg}</div>{/if}

		<div class="stats">
			<div class="stats-hdr"><Users size={18} /><h2>Usage Statistics</h2></div>
			<div class="stat-nums">
				<span>{coupon.usage_count} used</span>
				<span>{coupon.usage_limit != null ? `of ${coupon.usage_limit}` : 'Unlimited'}</span>
			</div>
			{#if coupon.usage_limit != null}
				<div class="bar-bg"><div class="bar-fill" style="width: {usagePercent}%"></div></div>
			{/if}
			{#if coupon.recent_usages && coupon.recent_usages.length > 0}
				<h3 class="usage-lbl">Recent Usages</h3>
				<div class="usage-list">
					{#each coupon.recent_usages as usage (usage.used_at + usage.user_email)}
						<div class="usage-row">
							<span class="u-email">{usage.user_email}</span>
							<span class="u-amt">${usage.amount.toFixed(2)}</span>
							<span class="u-date">{new Date(usage.used_at).toLocaleDateString()}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<form onsubmit={handleUpdate} class="card">
			<div class="cols">
				<div class="col">
					<div class="fld">
						<label for="code">Coupon Code</label>
						<input id="code" type="text" value={coupon.code} readonly class="ro" />
					</div>
					<div class="fld">
						<label for="description">Description</label>
						<textarea id="description" bind:value={description} rows="3" placeholder="Internal note about this coupon..."></textarea>
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
						<label for="value">Value {discountType === 'percentage' ? '(%)' : discountType === 'fixed' ? '($)' : '(days)'}</label>
						<input id="value" type="number" step="any" min="0" bind:value={value} required />
					</div>
					<div class="fld">
						<label for="minPurchase">Min Purchase ($)</label>
						<input id="minPurchase" type="number" step="0.01" min="0" bind:value={minPurchase} placeholder="0.00" />
					</div>
					<div class="fld">
						<label for="maxDiscount">Max Discount ($)</label>
						<input id="maxDiscount" type="number" step="0.01" min="0" bind:value={maxDiscount} placeholder="No limit" />
					</div>
				</div>

				<div class="col">
					<div class="fld">
						<label for="usageLimit">Usage Limit</label>
						<input id="usageLimit" type="number" min="0" bind:value={usageLimit} placeholder="Unlimited" />
					</div>
					<div class="fld">
						<label for="perUserLimit">Per-User Limit</label>
						<input id="perUserLimit" type="number" min="0" bind:value={perUserLimit} placeholder="Unlimited" />
					</div>
					<div class="fld">
						<label for="startDate">Start Date</label>
						<input id="startDate" type="date" bind:value={startDate} />
					</div>
					<div class="fld">
						<label for="expiryDate">Expiry Date</label>
						<input id="expiryDate" type="date" bind:value={expiryDate} />
					</div>
					<label class="tog"><input type="checkbox" bind:checked={stackable} /><span class="track"><span class="thumb"></span></span><span>Stackable</span></label>
					<label class="tog"><input type="checkbox" bind:checked={firstPurchaseOnly} /><span class="track"><span class="thumb"></span></span><span>First Purchase Only</span></label>
					<label class="tog"><input type="checkbox" bind:checked={active} /><span class="track"><span class="thumb"></span></span><span>Active</span></label>
				</div>
			</div>

			<div class="actions">
				<button type="button" onclick={handleDelete} disabled={deleting} class="del-btn">
					<Trash size={16} weight="bold" /> {deleting ? 'Deleting...' : 'Delete'}
				</button>
				<div class="actions-r">
					<a href="/admin/coupons" class="cancel">Cancel</a>
					<button type="submit" disabled={saving} class="submit">
						<FloppyDisk size={16} weight="bold" /> {saving ? 'Saving...' : 'Update Coupon'}
					</button>
				</div>
			</div>
		</form>
	{/if}
</div>

<style>
	.back { display: inline-flex; align-items: center; gap: .5rem; color: var(--color-grey-400); font-size: var(--fs-sm); text-decoration: none; margin-bottom: 1.5rem; transition: color 200ms var(--ease-out); }
	.back:hover { color: var(--color-white); }
	.status { text-align: center; padding: 3rem; color: var(--color-grey-400); }
	.hdr { display: flex; align-items: center; gap: .75rem; color: var(--color-teal); margin-bottom: 1.5rem; }
	.hdr h1 { font-size: var(--fs-2xl); font-weight: var(--w-bold); color: var(--color-white); font-family: var(--font-heading); }
	.badge { margin-left: auto; padding: .35rem .85rem; background: rgba(15,164,175,.12); border: 1px solid rgba(15,164,175,.3); border-radius: var(--radius-full); color: var(--color-teal-light); font-size: var(--fs-sm); font-weight: var(--w-semibold); font-family: monospace; letter-spacing: .05em; }
	.err { background: rgba(239,68,68,.1); border: 1px solid rgba(239,68,68,.3); color: #fca5a5; padding: .75rem 1rem; border-radius: var(--radius-lg); font-size: var(--fs-sm); margin-bottom: 1rem; }
	.ok { background: rgba(34,197,94,.1); border: 1px solid rgba(34,197,94,.3); color: #86efac; padding: .75rem 1rem; border-radius: var(--radius-lg); font-size: var(--fs-sm); margin-bottom: 1rem; }
	.stats { background: var(--color-navy-mid); border: 1px solid rgba(255,255,255,.06); border-radius: var(--radius-xl); padding: 1.5rem; margin-bottom: 1.5rem; }
	.stats-hdr { display: flex; align-items: center; gap: .5rem; color: var(--color-grey-300); margin-bottom: 1rem; }
	.stats-hdr h2 { font-size: var(--fs-lg); font-weight: var(--w-bold); color: var(--color-white); font-family: var(--font-heading); }
	.stat-nums { display: flex; justify-content: space-between; font-size: var(--fs-sm); color: var(--color-grey-400); margin-bottom: .5rem; }
	.bar-bg { width: 100%; height: .5rem; background: rgba(255,255,255,.08); border-radius: var(--radius-full); overflow: hidden; }
	.bar-fill { height: 100%; background: linear-gradient(90deg, var(--color-teal), var(--color-teal-light)); border-radius: var(--radius-full); transition: width 300ms var(--ease-out); }
	.usage-lbl { font-size: var(--fs-xs); font-weight: var(--w-semibold); color: var(--color-grey-400); text-transform: uppercase; letter-spacing: .05em; margin: 1.25rem 0 .75rem; }
	.usage-list { display: flex; flex-direction: column; gap: .35rem; }
	.usage-row { display: flex; align-items: center; gap: 1rem; padding: .5rem .75rem; background: rgba(255,255,255,.03); border-radius: var(--radius-md); font-size: var(--fs-sm); }
	.u-email { flex: 1; color: var(--color-grey-300); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.u-amt { color: var(--color-green); font-weight: var(--w-semibold); }
	.u-date { color: var(--color-grey-500); }
	.card { background: var(--color-navy-mid); border: 1px solid rgba(255,255,255,.06); border-radius: var(--radius-xl); padding: 2rem; }
	.cols { display: grid; grid-template-columns: 1fr 1fr; gap: 2rem; margin-bottom: 2rem; }
	.col { display: flex; flex-direction: column; gap: 1.25rem; }
	.fld { display: flex; flex-direction: column; gap: .5rem; }
	.fld label { font-size: var(--fs-sm); font-weight: var(--w-medium); color: var(--color-grey-300); }
	input, textarea, select { width: 100%; padding: .65rem .85rem; background: rgba(255,255,255,.05); border: 1px solid rgba(255,255,255,.1); border-radius: var(--radius-lg); color: var(--color-white); font-size: var(--fs-sm); font-family: inherit; transition: border-color 200ms var(--ease-out); }
	input:focus, textarea:focus, select:focus { outline: none; border-color: var(--color-teal); }
	input::placeholder, textarea::placeholder { color: var(--color-grey-500); }
	textarea { resize: vertical; }
	.ro { opacity: .6; cursor: not-allowed; border-style: dashed; }
	.tog { display: flex; align-items: center; gap: .75rem; font-size: var(--fs-sm); color: var(--color-grey-300); cursor: pointer; padding: .4rem 0; }
	.tog input[type='checkbox'] { position: absolute; opacity: 0; width: 0; height: 0; }
	.track { position: relative; width: 2.5rem; height: 1.35rem; background: rgba(255,255,255,.1); border-radius: var(--radius-full); flex-shrink: 0; transition: background 200ms var(--ease-out); }
	.tog input:checked + .track { background: var(--color-teal); }
	.thumb { position: absolute; top: .15rem; left: .15rem; width: 1.05rem; height: 1.05rem; background: var(--color-white); border-radius: var(--radius-full); transition: transform 200ms var(--ease-out); }
	.tog input:checked + .track .thumb { transform: translateX(1.15rem); }
	.actions { display: flex; align-items: center; justify-content: space-between; padding-top: 1rem; border-top: 1px solid rgba(255,255,255,.06); }
	.actions-r { display: flex; gap: 1rem; align-items: center; }
	.del-btn { display: flex; align-items: center; gap: .4rem; padding: .6rem 1.25rem; background: rgba(239,68,68,.08); border: 1px solid rgba(239,68,68,.25); border-radius: var(--radius-lg); color: var(--color-red); font-size: var(--fs-sm); font-weight: var(--w-semibold); cursor: pointer; transition: background 200ms var(--ease-out); }
	.del-btn:hover:not(:disabled) { background: rgba(239,68,68,.18); }
	.del-btn:disabled { opacity: .5; cursor: not-allowed; }
	.cancel { padding: .6rem 1.25rem; border: 1px solid rgba(255,255,255,.1); border-radius: var(--radius-lg); color: var(--color-grey-400); font-size: var(--fs-sm); font-weight: var(--w-medium); text-decoration: none; transition: border-color 200ms var(--ease-out), color 200ms var(--ease-out); }
	.cancel:hover { border-color: rgba(255,255,255,.2); color: var(--color-white); }
	.submit { display: flex; align-items: center; gap: .4rem; padding: .6rem 1.5rem; background: linear-gradient(135deg, var(--color-teal), #0d8a94); color: var(--color-white); font-weight: var(--w-semibold); font-size: var(--fs-sm); border-radius: var(--radius-lg); cursor: pointer; transition: opacity 200ms var(--ease-out); }
	.submit:hover:not(:disabled) { opacity: .9; }
	.submit:disabled { opacity: .5; cursor: not-allowed; }
	@media (max-width: 768px) { .cols { grid-template-columns: 1fr; gap: 1.25rem; } }
</style>
