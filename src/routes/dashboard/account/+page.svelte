<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { SubscriptionStatusResponse, BillingPortalResponse, CouponValidationResponse } from '$lib/api/types';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';
	import LockIcon from 'phosphor-svelte/lib/LockIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import TagIcon from 'phosphor-svelte/lib/TagIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';

	let name = $state(auth.user?.name ?? '');
	let saving = $state(false);
	let saved = $state(false);
	let profileError = $state('');

	let curPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let pwSaving = $state(false);
	let pwMsg = $state('');
	let pwError = $state('');

	let subscription = $state<SubscriptionStatusResponse | null>(null);
	let billingBusy = $state(false);
	let subMsg = $state('');

	let couponCode = $state('');
	let couponBusy = $state(false);
	let couponMsg = $state('');
	let couponError = $state('');

	let deleteConfirm = $state('');
	let deleting = $state(false);
	let deleteError = $state('');

	onMount(async () => {
		try { subscription = await api.get<SubscriptionStatusResponse>('/api/member/subscription'); } catch { /* silent */ }
	});

	async function handleProfileSave(e: Event) {
		e.preventDefault();
		saving = true; saved = false; profileError = '';
		try {
			const res = await api.put<{ id: string; email: string; name: string; role: string; avatar_url: string | null; created_at: string }>('/api/member/profile', { name });
			auth.setUser({ ...auth.user!, name: res.name });
			saved = true;
			setTimeout(() => (saved = false), 3000);
		} catch (err) { profileError = err instanceof ApiError ? err.message : 'Failed to update profile'; }
		finally { saving = false; }
	}

	async function handlePasswordChange(e: Event) {
		e.preventDefault();
		pwError = ''; pwMsg = '';
		if (newPassword !== confirmPassword) { pwError = 'Passwords do not match'; return; }
		if (newPassword.length < 8) { pwError = 'Password must be at least 8 characters'; return; }
		pwSaving = true;
		try {
			await api.post('/api/member/password', { current_password: curPassword, new_password: newPassword });
			pwMsg = 'Password updated successfully';
			curPassword = ''; newPassword = ''; confirmPassword = '';
			setTimeout(() => (pwMsg = ''), 3000);
		} catch (err) { pwError = err instanceof ApiError ? err.message : 'Failed to change password'; }
		finally { pwSaving = false; }
	}

	function formatDate(d: string): string {
		return new Date(d).toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' });
	}

	function planLabel(plan: string): string { return plan === 'annual' ? 'Annual ($399/yr)' : 'Monthly ($49/mo)'; }
	function statusColor(s: string): string { return s === 'active' || s === 'trialing' ? 'var(--color-green)' : s === 'past_due' ? 'var(--color-gold)' : 'var(--color-red)'; }

	async function openBilling() {
		billingBusy = true; subMsg = '';
		try { const { url } = await api.post<BillingPortalResponse>('/api/member/billing-portal', {}); window.location.href = url; }
		catch (err) { subMsg = err instanceof ApiError ? err.message : 'Could not open billing portal'; }
		finally { billingBusy = false; }
	}

	async function applyCoupon() {
		couponError = ''; couponMsg = '';
		if (!couponCode.trim()) { couponError = 'Enter a coupon code'; return; }
		couponBusy = true;
		try {
			const res = await api.post<CouponValidationResponse>('/api/member/coupons/apply', { code: couponCode.trim() });
			if (res.valid) { couponMsg = res.message || 'Coupon applied successfully'; couponCode = ''; }
			else { couponError = res.message || 'Invalid coupon code'; }
		} catch (err) { couponError = err instanceof ApiError ? err.message : 'Failed to apply coupon'; }
		finally { couponBusy = false; }
	}

	async function handleDelete() {
		if (deleteConfirm !== 'DELETE') { deleteError = 'Type DELETE to confirm'; return; }
		deleteError = '';
		deleting = true;
		try {
			await api.del('/api/member/account');
			auth.logout();
			goto('/');
		} catch (err) { deleteError = err instanceof ApiError ? err.message : 'Failed to delete account'; }
		finally { deleting = false; }
	}
</script>

<svelte:head><title>Account - Precision Options Signals</title></svelte:head>

<div class="acct">
	<h2 class="acct-title">Account Settings</h2>

	<!-- Profile -->
	<section class="card">
		<div class="card-head"><UserCircleIcon size={22} weight="duotone" color="var(--color-teal)" /><h3>Profile</h3></div>
		<form onsubmit={handleProfileSave} class="form">
			<div class="field"><label for="name" class="label">Name</label><input id="name" type="text" bind:value={name} class="input" /></div>
			<div class="field"><span class="label">Email</span><p class="static">{auth.user?.email}</p></div>
			{#if profileError}<p class="err">{profileError}</p>{/if}
			<button type="submit" disabled={saving} class="btn-primary">{saving ? 'Saving...' : saved ? 'Saved!' : 'Save Changes'}</button>
		</form>
	</section>

	<!-- Change Password -->
	<section class="card">
		<div class="card-head"><LockIcon size={22} weight="duotone" color="var(--color-teal)" /><h3>Change Password</h3></div>
		<form onsubmit={handlePasswordChange} class="form">
			<div class="field"><label for="cur-pw" class="label">Current Password</label><input id="cur-pw" type="password" bind:value={curPassword} class="input" autocomplete="current-password" /></div>
			<div class="field"><label for="new-pw" class="label">New Password</label><input id="new-pw" type="password" bind:value={newPassword} class="input" autocomplete="new-password" /></div>
			<div class="field"><label for="confirm-pw" class="label">Confirm New Password</label><input id="confirm-pw" type="password" bind:value={confirmPassword} class="input" autocomplete="new-password" /></div>
			{#if pwError}<p class="err">{pwError}</p>{/if}
			{#if pwMsg}<p class="success">{pwMsg}</p>{/if}
			<button type="submit" disabled={pwSaving} class="btn-primary">{pwSaving ? 'Updating...' : 'Update Password'}</button>
		</form>
	</section>

	<!-- Subscription -->
	<section class="card">
		<div class="card-head"><LightningIcon size={22} weight="duotone" color="var(--color-teal)" /><h3>Subscription</h3></div>
		{#if subscription?.subscription}
			{@const sub = subscription.subscription}
			<div class="sub-rows">
				<div class="sub-row"><span class="sub-label">Plan</span><span class="sub-val">{planLabel(sub.plan)}</span></div>
				<div class="sub-row"><span class="sub-label">Status</span><span class="sub-status" style="color:{statusColor(sub.status)}">{sub.status}</span></div>
				<div class="sub-row"><span class="sub-label">Next Billing</span><span class="sub-val">{formatDate(sub.current_period_end)}</span></div>
			</div>
			<button type="button" class="btn-primary" style="margin-top:1rem" disabled={billingBusy} onclick={openBilling}>
				{billingBusy ? 'Opening...' : 'Manage Billing'}
			</button>
			{#if subMsg}<p class="info">{subMsg}</p>{/if}
		{:else}
			<p class="muted">No active subscription.</p>
			<a href="/pricing/monthly" class="btn-primary" style="display:inline-block;margin-top:.75rem;text-decoration:none;text-align:center">View Plans</a>
		{/if}
	</section>

	<!-- Coupon -->
	<section class="card">
		<div class="card-head"><TagIcon size={22} weight="duotone" color="var(--color-teal)" /><h3>Coupon</h3></div>
		<div class="coupon-row">
			<input type="text" placeholder="Enter coupon code" bind:value={couponCode} class="input" />
			<button type="button" class="btn-primary" disabled={couponBusy} onclick={applyCoupon}>{couponBusy ? 'Applying...' : 'Apply'}</button>
		</div>
		{#if couponError}<p class="err">{couponError}</p>{/if}
		{#if couponMsg}<p class="success">{couponMsg}</p>{/if}
	</section>

	<!-- Danger Zone -->
	<section class="card card--danger">
		<div class="card-head"><WarningIcon size={22} weight="duotone" color="var(--color-red)" /><h3 class="danger-title">Delete Account</h3></div>
		<p class="muted">This action is permanent. All your data, progress, and subscription will be deleted.</p>
		<div class="delete-row">
			<input type="text" placeholder='Type "DELETE" to confirm' bind:value={deleteConfirm} class="input" />
			<button type="button" class="btn-danger" disabled={deleting || deleteConfirm !== 'DELETE'} onclick={handleDelete}>
				{deleting ? 'Deleting...' : 'Delete Account'}
			</button>
		</div>
		{#if deleteError}<p class="err">{deleteError}</p>{/if}
	</section>
</div>

<style>
	.acct-title { font-size:var(--fs-2xl); font-weight:var(--w-bold); color:var(--color-white); font-family:var(--font-heading); margin-bottom:2rem; }
	.card { background:var(--color-navy-mid); border:1px solid rgba(255,255,255,.06); border-radius:var(--radius-xl); padding:1.5rem; margin-bottom:1.5rem; }
	.card--danger { border-color:rgba(224,72,72,.2); }
	.card-head { display:flex; align-items:center; gap:.65rem; margin-bottom:1.25rem; padding-bottom:1rem; border-bottom:1px solid rgba(255,255,255,.06); }
	.card-head h3 { font-size:var(--fs-lg); font-weight:var(--w-bold); color:var(--color-white); }
	.danger-title { color:var(--color-red) !important; }
	.form { display:flex; flex-direction:column; gap:1rem; }
	.field { display:flex; flex-direction:column; gap:.4rem; }
	.label { font-size:var(--fs-xs); font-weight:var(--w-medium); color:var(--color-grey-400); text-transform:uppercase; letter-spacing:.05em; }
	.input { padding:.65rem .85rem; background:rgba(255,255,255,.05); border:1px solid rgba(255,255,255,.1); border-radius:var(--radius-lg); color:var(--color-white); font-size:var(--fs-sm); transition:border-color 200ms var(--ease-out); }
	.input:focus { outline:none; border-color:var(--color-teal); }
	.static { color:var(--color-grey-300); font-size:var(--fs-sm); }
	.btn-primary { align-self:flex-start; padding:.6rem 1.5rem; background:linear-gradient(135deg,var(--color-teal),#0d8a94); color:var(--color-white); font-weight:var(--w-semibold); font-size:var(--fs-sm); border:none; border-radius:var(--radius-lg); cursor:pointer; transition:opacity 200ms var(--ease-out); }
	.btn-primary:hover:not(:disabled){ opacity:.9; }
	.btn-primary:disabled { opacity:.5; cursor:not-allowed; }
	.btn-danger { padding:.6rem 1.5rem; background:var(--color-red); color:var(--color-white); font-weight:var(--w-semibold); font-size:var(--fs-sm); border:none; border-radius:var(--radius-lg); cursor:pointer; transition:opacity 200ms var(--ease-out); }
	.btn-danger:hover:not(:disabled){ opacity:.9; }
	.btn-danger:disabled { opacity:.5; cursor:not-allowed; }
	.err { color:#fca5a5; font-size:var(--fs-sm); }
	.success { color:var(--color-green); font-size:var(--fs-sm); }
	.info { color:var(--color-teal-light); font-size:var(--fs-xs); margin-top:.5rem; }
	.muted { color:var(--color-grey-400); font-size:var(--fs-sm); line-height:var(--lh-relaxed); }
	.sub-rows { display:flex; flex-direction:column; gap:.85rem; }
	.sub-row { display:flex; justify-content:space-between; align-items:center; }
	.sub-label { font-size:var(--fs-sm); color:var(--color-grey-400); }
	.sub-val { font-size:var(--fs-sm); font-weight:var(--w-semibold); color:var(--color-white); }
	.sub-status { font-size:var(--fs-sm); font-weight:var(--w-bold); text-transform:capitalize; }
	.coupon-row { display:flex; gap:.75rem; align-items:stretch; }
	.coupon-row .input { flex:1; }
	.delete-row { display:flex; gap:.75rem; align-items:stretch; margin-top:1rem; }
	.delete-row .input { flex:1; }
</style>
