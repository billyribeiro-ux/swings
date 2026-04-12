<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type {
		BillingPortalResponse,
		SubscriptionStatusResponse,
		CouponValidationResponse
	} from '$lib/api/types';
	import UserCircle from 'phosphor-svelte/lib/UserCircle';
	import Lightning from 'phosphor-svelte/lib/Lightning';
	import Lock from 'phosphor-svelte/lib/Lock';
	import Tag from 'phosphor-svelte/lib/Tag';
	import Warning from 'phosphor-svelte/lib/Warning';
	import Trash from 'phosphor-svelte/lib/Trash';

	// Profile
	let name = $state(auth.user?.name ?? '');
	let saving = $state(false);
	let saved = $state(false);
	let profileError = $state('');

	// Subscription
	let subscription = $state<SubscriptionStatusResponse | null>(null);
	let billingBusy = $state(false);
	let subActionBusy = $state(false);
	let subMessage = $state('');

	// Password
	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let passwordSaving = $state(false);
	let passwordSaved = $state(false);
	let passwordError = $state('');

	// Coupon
	let couponCode = $state('');
	let couponApplying = $state(false);
	let couponMessage = $state('');
	let couponSuccess = $state(false);

	// Delete Account
	let deleteConfirm = $state(false);
	let deleteText = $state('');
	let deleting = $state(false);
	let deleteError = $state('');

	onMount(async () => {
		try {
			subscription = await api.get<SubscriptionStatusResponse>('/api/member/subscription');
		} catch {
			// silently handle
		}
	});

	// -- Profile --
	async function handleSaveProfile(e: Event) {
		e.preventDefault();
		saving = true;
		saved = false;
		profileError = '';

		try {
			const res = await api.put<{
				id: string;
				email: string;
				name: string;
				role: string;
				avatar_url: string | null;
				created_at: string;
			}>('/api/member/profile', { name });
			auth.setUser({
				...auth.user!,
				name: res.name
			});
			saved = true;
			setTimeout(() => (saved = false), 3000);
		} catch (err) {
			profileError = err instanceof ApiError ? err.message : 'Failed to update profile';
		} finally {
			saving = false;
		}
	}

	// -- Password --
	async function handleChangePassword(e: Event) {
		e.preventDefault();
		passwordError = '';
		passwordSaved = false;

		if (newPassword.length < 8) {
			passwordError = 'New password must be at least 8 characters.';
			return;
		}
		if (newPassword !== confirmPassword) {
			passwordError = 'Passwords do not match.';
			return;
		}

		passwordSaving = true;
		try {
			await api.put('/api/member/password', {
				current_password: currentPassword,
				new_password: newPassword
			});
			passwordSaved = true;
			currentPassword = '';
			newPassword = '';
			confirmPassword = '';
			setTimeout(() => (passwordSaved = false), 3000);
		} catch (err) {
			passwordError = err instanceof ApiError ? err.message : 'Failed to change password';
		} finally {
			passwordSaving = false;
		}
	}

	// -- Subscription --
	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
	}

	async function openBillingPortal() {
		subMessage = '';
		billingBusy = true;
		try {
			const { url } = await api.post<BillingPortalResponse>('/api/member/billing-portal', {});
			window.location.href = url;
		} catch (e) {
			subMessage = e instanceof ApiError ? e.message : 'Could not open billing portal';
		} finally {
			billingBusy = false;
		}
	}

	function planLabel(plan: string): string {
		return plan === 'annual' ? 'Annual ($399/yr)' : 'Monthly ($49/mo)';
	}

	function statusColor(status: string): string {
		if (status === 'active' || status === 'trialing') return '#22c55e';
		if (status === 'past_due') return '#f59e0b';
		return '#ef4444';
	}

	// -- Coupon --
	async function handleApplyCoupon(e: Event) {
		e.preventDefault();
		if (!couponCode.trim()) return;

		couponApplying = true;
		couponMessage = '';
		couponSuccess = false;

		try {
			const res = await api.post<CouponValidationResponse>('/api/member/coupons/apply', {
				code: couponCode.trim()
			});
			if (res.valid) {
				couponMessage = res.message || 'Coupon applied successfully!';
				couponSuccess = true;
				couponCode = '';
			} else {
				couponMessage = res.message || 'Invalid coupon code.';
				couponSuccess = false;
			}
		} catch (err) {
			couponMessage = err instanceof ApiError ? err.message : 'Failed to apply coupon';
			couponSuccess = false;
		} finally {
			couponApplying = false;
		}
	}

	// -- Delete Account --
	async function handleDeleteAccount() {
		if (deleteText !== 'DELETE') return;
		deleting = true;
		deleteError = '';

		try {
			await api.delete('/api/member/account');
			auth.logout();
			goto('/');
		} catch (err) {
			deleteError = err instanceof ApiError ? err.message : 'Failed to delete account';
		} finally {
			deleting = false;
		}
	}
</script>

<svelte:head>
	<title>Account - Explosive Swings</title>
</svelte:head>

<div class="account">
	<h2 class="account__title">Account Settings</h2>

	<!-- Profile Section -->
	<section class="account__section">
		<div class="account__section-header">
			<UserCircle size={22} weight="duotone" color="var(--color-teal)" />
			<h3 class="account__section-title">Profile</h3>
		</div>

		<form onsubmit={handleSaveProfile} class="account__form">
			<div class="account__field">
				<label for="name" class="account__label">Name</label>
				<input id="name" type="text" bind:value={name} class="account__input" />
			</div>

			<div class="account__field">
				<span class="account__label">Email</span>
				<p class="account__static">{auth.user?.email}</p>
			</div>

			<div class="account__field">
				<span class="account__label">Member Since</span>
				<p class="account__static">
					{auth.user?.created_at ? formatDate(auth.user.created_at) : '-'}
				</p>
			</div>

			{#if profileError}
				<p class="account__error">{profileError}</p>
			{/if}

			<button type="submit" disabled={saving} class="account__save">
				{#if saving}
					Saving...
				{:else if saved}
					Saved!
				{:else}
					Save Changes
				{/if}
			</button>
		</form>
	</section>

	<!-- Change Password Section -->
	<section class="account__section">
		<div class="account__section-header">
			<Lock size={22} weight="duotone" color="var(--color-teal)" />
			<h3 class="account__section-title">Change Password</h3>
		</div>

		<form onsubmit={handleChangePassword} class="account__form">
			<div class="account__field">
				<label for="current-password" class="account__label">Current Password</label>
				<input
					id="current-password"
					type="password"
					bind:value={currentPassword}
					class="account__input"
					autocomplete="current-password"
				/>
			</div>

			<div class="account__field">
				<label for="new-password" class="account__label">New Password</label>
				<input
					id="new-password"
					type="password"
					bind:value={newPassword}
					class="account__input"
					autocomplete="new-password"
				/>
			</div>

			<div class="account__field">
				<label for="confirm-password" class="account__label">Confirm New Password</label>
				<input
					id="confirm-password"
					type="password"
					bind:value={confirmPassword}
					class="account__input"
					autocomplete="new-password"
				/>
			</div>

			{#if passwordError}
				<p class="account__error">{passwordError}</p>
			{/if}

			<button type="submit" disabled={passwordSaving} class="account__save">
				{#if passwordSaving}
					Updating...
				{:else if passwordSaved}
					Password Updated!
				{:else}
					Update Password
				{/if}
			</button>
		</form>
	</section>

	<!-- Subscription Section -->
	<section class="account__section">
		<div class="account__section-header">
			<Lightning size={22} weight="duotone" color="var(--color-teal)" />
			<h3 class="account__section-title">Subscription</h3>
		</div>

		{#if subscription?.subscription}
			{@const sub = subscription.subscription}
			<div class="sub-card">
				<div class="sub-card__row">
					<span class="sub-card__label">Plan</span>
					<span class="sub-card__value">{planLabel(sub.plan)}</span>
				</div>
				<div class="sub-card__row">
					<span class="sub-card__label">Status</span>
					<span class="sub-card__status" style="color: {statusColor(sub.status)}">
						{sub.status}
					</span>
				</div>
				<div class="sub-card__row">
					<span class="sub-card__label">Next Billing Date</span>
					<span class="sub-card__value">
						{formatDate(sub.current_period_end)}
					</span>
				</div>
				<div class="sub-card__actions">
					<button
						type="button"
						class="sub-card__btn sub-card__btn--primary"
						disabled={billingBusy || subActionBusy}
						onclick={openBillingPortal}
					>
						{billingBusy ? 'Opening...' : 'Manage Billing'}
					</button>
				</div>
				{#if subMessage}
					<p class="sub-card__msg">{subMessage}</p>
				{/if}
			</div>
		{:else}
			<div class="sub-card sub-card--empty">
				<p>No active subscription.</p>
				<a href="/#pricing" class="sub-card__cta">View Plans</a>
			</div>
		{/if}
	</section>

	<!-- Coupon Section -->
	<section class="account__section">
		<div class="account__section-header">
			<Tag size={22} weight="duotone" color="var(--color-teal)" />
			<h3 class="account__section-title">Coupon Code</h3>
		</div>

		<form onsubmit={handleApplyCoupon} class="account__coupon-form">
			<div class="account__coupon-row">
				<input
					type="text"
					placeholder="Enter coupon code"
					bind:value={couponCode}
					class="account__input account__coupon-input"
				/>
				<button type="submit" disabled={couponApplying || !couponCode.trim()} class="account__save">
					{couponApplying ? 'Applying...' : 'Apply'}
				</button>
			</div>
			{#if couponMessage}
				<p class="account__coupon-msg" class:account__coupon-msg--success={couponSuccess}>
					{couponMessage}
				</p>
			{/if}
		</form>
	</section>

	<!-- Danger Zone -->
	<section class="account__section account__section--danger">
		<div class="account__section-header">
			<Warning size={22} weight="duotone" color="var(--color-red)" />
			<h3 class="account__section-title account__section-title--danger">Danger Zone</h3>
		</div>

		{#if !deleteConfirm}
			<div class="danger-zone">
				<div>
					<p class="danger-zone__title">Delete Account</p>
					<p class="danger-zone__desc">
						Permanently delete your account and all associated data. This action cannot be undone.
					</p>
				</div>
				<button class="danger-zone__btn" onclick={() => (deleteConfirm = true)}>
					<Trash size={16} />
					Delete Account
				</button>
			</div>
		{:else}
			<div class="danger-zone danger-zone--confirm">
				<p class="danger-zone__warning">
					This will permanently delete your account, enrollments, and progress. Type
					<strong>DELETE</strong> to confirm.
				</p>
				<div class="danger-zone__confirm-row">
					<input
						type="text"
						bind:value={deleteText}
						placeholder="Type DELETE"
						class="account__input danger-zone__input"
					/>
					<button
						class="danger-zone__btn danger-zone__btn--confirm"
						disabled={deleteText !== 'DELETE' || deleting}
						onclick={handleDeleteAccount}
					>
						{deleting ? 'Deleting...' : 'Confirm Delete'}
					</button>
					<button class="danger-zone__btn danger-zone__btn--cancel" onclick={() => { deleteConfirm = false; deleteText = ''; }}>
						Cancel
					</button>
				</div>
				{#if deleteError}
					<p class="account__error">{deleteError}</p>
				{/if}
			</div>
		{/if}
	</section>
</div>

<style>
	.account {
		max-width: var(--container-sm);
	}

	.account__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 2rem;
	}

	.account__section {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.account__section--danger {
		border-color: rgba(224, 72, 72, 0.2);
	}

	.account__section-header {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		margin-bottom: 1.25rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.account__section-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.account__section-title--danger {
		color: var(--color-red);
	}

	.account__form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.account__field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.account__label {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.account__input {
		padding: 0.65rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		transition: border-color 200ms var(--ease-out);
	}

	.account__input::placeholder {
		color: var(--color-grey-500);
	}

	.account__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.account__static {
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
	}

	.account__error {
		color: #fca5a5;
		font-size: var(--fs-sm);
	}

	.account__save {
		align-self: flex-start;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}

	.account__save:hover:not(:disabled) {
		opacity: 0.9;
	}

	.account__save:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Coupon */
	.account__coupon-form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.account__coupon-row {
		display: flex;
		gap: 0.75rem;
		align-items: stretch;
	}

	.account__coupon-input {
		flex: 1;
	}

	.account__coupon-msg {
		font-size: var(--fs-xs);
		color: #fca5a5;
	}

	.account__coupon-msg--success {
		color: var(--color-green);
	}

	/* Subscription */
	.sub-card {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.sub-card--empty {
		align-items: center;
		text-align: center;
		color: var(--color-grey-400);
		gap: 1rem;
	}

	.sub-card__cta {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		text-decoration: none;
	}

	.sub-card__row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.sub-card__label {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
	}

	.sub-card__value {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.sub-card__status {
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
		text-transform: capitalize;
	}

	.sub-card__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.sub-card__btn {
		padding: 0.45rem 0.85rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		border-radius: var(--radius-md);
		border: 1px solid rgba(255, 255, 255, 0.15);
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-200);
		cursor: pointer;
	}

	.sub-card__btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.sub-card__btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		border-color: transparent;
		color: var(--color-white);
	}

	.sub-card__msg {
		margin: 0.5rem 0 0;
		font-size: var(--fs-xs);
		color: var(--color-teal-light);
		line-height: 1.45;
	}

	/* Danger Zone */
	.danger-zone {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1.5rem;
		flex-wrap: wrap;
	}

	.danger-zone--confirm {
		flex-direction: column;
		align-items: stretch;
	}

	.danger-zone__title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin-bottom: 0.25rem;
	}

	.danger-zone__desc {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		line-height: var(--lh-relaxed);
	}

	.danger-zone__warning {
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		line-height: var(--lh-relaxed);
		margin-bottom: 1rem;
	}

	.danger-zone__warning strong {
		color: var(--color-red);
	}

	.danger-zone__confirm-row {
		display: flex;
		gap: 0.75rem;
		align-items: stretch;
		flex-wrap: wrap;
	}

	.danger-zone__input {
		flex: 1;
		min-width: 8rem;
	}

	.danger-zone__btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.5rem 1rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		border-radius: var(--radius-lg);
		border: 1px solid rgba(224, 72, 72, 0.3);
		background-color: rgba(224, 72, 72, 0.1);
		color: var(--color-red);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
		flex-shrink: 0;
	}

	.danger-zone__btn:hover {
		background-color: rgba(224, 72, 72, 0.2);
	}

	.danger-zone__btn--confirm {
		background-color: var(--color-red);
		border-color: var(--color-red);
		color: var(--color-white);
	}

	.danger-zone__btn--confirm:hover {
		opacity: 0.9;
		background-color: var(--color-red);
	}

	.danger-zone__btn--confirm:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.danger-zone__btn--cancel {
		background-color: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.15);
		color: var(--color-grey-300);
	}

	.danger-zone__btn--cancel:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}
</style>
