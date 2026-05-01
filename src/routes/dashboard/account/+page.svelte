<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type {
		SubscriptionStatusResponse,
		BillingPortalResponse,
		CouponValidationResponse
	} from '$lib/api/types';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';
	import LockIcon from 'phosphor-svelte/lib/LockIcon';
	import LightningIcon from 'phosphor-svelte/lib/LightningIcon';
	import BellIcon from 'phosphor-svelte/lib/BellIcon';

	type TabKey = 'profile' | 'security' | 'subscription' | 'notifications';
	type NotificationPreferences = {
		new_watchlist: boolean;
		new_course: boolean;
		marketing: boolean;
	};

	let activeTab = $state<TabKey>('profile');

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
	let cancelBusy = $state(false);
	let resumeBusy = $state(false);

	let couponCode = $state('');
	let couponBusy = $state(false);
	let couponMsg = $state('');
	let couponError = $state('');

	let deleteConfirm = $state('');
	let deleting = $state(false);
	let deleteError = $state('');

	let notifPrefs = $state<NotificationPreferences>({
		new_watchlist: true,
		new_course: true,
		marketing: true
	});
	let notifLoaded = $state(false);
	let notifLoading = $state(false);
	let notifSaving = $state(false);
	let notifMsg = $state('');
	let notifError = $state('');

	onMount(async () => {
		try {
			subscription = await api.get<SubscriptionStatusResponse>('/api/member/subscription');
		} catch {
			/* silent */
		}
	});

	async function loadNotificationPreferences() {
		if (notifLoaded || notifLoading) return;
		notifLoading = true;
		try {
			const res = await api.get<NotificationPreferences>(
				'/api/member/notification-preferences'
			);
			notifPrefs = {
				new_watchlist: res.new_watchlist ?? true,
				new_course: res.new_course ?? true,
				marketing: res.marketing ?? true
			};
		} catch {
			notifPrefs = { new_watchlist: true, new_course: true, marketing: true };
		} finally {
			notifLoaded = true;
			notifLoading = false;
		}
	}

	function selectTab(tab: TabKey) {
		activeTab = tab;
		if (tab === 'notifications') {
			void loadNotificationPreferences();
		}
	}

	async function handleProfileSave(e: Event) {
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
			auth.setUser({ ...auth.user!, name: res.name });
			saved = true;
			setTimeout(() => (saved = false), 3000);
		} catch (err) {
			profileError = err instanceof ApiError ? err.message : 'Failed to update profile';
		} finally {
			saving = false;
		}
	}

	async function handlePasswordChange(e: Event) {
		e.preventDefault();
		pwError = '';
		pwMsg = '';
		if (newPassword !== confirmPassword) {
			pwError = 'Passwords do not match';
			return;
		}
		if (newPassword.length < 8) {
			pwError = 'Password must be at least 8 characters';
			return;
		}
		pwSaving = true;
		try {
			await api.post('/api/member/password', {
				current_password: curPassword,
				new_password: newPassword
			});
			pwMsg = 'Password updated successfully';
			curPassword = '';
			newPassword = '';
			confirmPassword = '';
			setTimeout(() => (pwMsg = ''), 3000);
		} catch (err) {
			pwError = err instanceof ApiError ? err.message : 'Failed to change password';
		} finally {
			pwSaving = false;
		}
	}

	function formatDate(d: string): string {
		return new Date(d).toLocaleDateString('en-US', {
			month: 'long',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function planLabel(plan: string): string {
		return plan === 'annual' ? 'Annual ($399/yr)' : 'Monthly ($49/mo)';
	}
	function statusColor(s: string): string {
		return s === 'active' || s === 'trialing'
			? 'var(--color-green)'
			: s === 'past_due'
				? 'var(--color-gold)'
				: 'var(--color-red)';
	}

	async function openBilling() {
		billingBusy = true;
		subMsg = '';
		try {
			const { url } = await api.post<BillingPortalResponse>('/api/member/billing-portal', {});
			window.location.href = url;
		} catch (err) {
			subMsg = err instanceof ApiError ? err.message : 'Could not open billing portal';
		} finally {
			billingBusy = false;
		}
	}

	async function cancelSubscription() {
		cancelBusy = true;
		subMsg = '';
		try {
			await api.post('/api/member/subscription/cancel', {});
			subscription = await api.get<SubscriptionStatusResponse>('/api/member/subscription');
			subMsg = 'Subscription set to cancel at period end';
		} catch (err) {
			subMsg = err instanceof ApiError ? err.message : 'Could not cancel subscription';
		} finally {
			cancelBusy = false;
		}
	}

	async function resumeSubscription() {
		resumeBusy = true;
		subMsg = '';
		try {
			await api.post('/api/member/subscription/resume', {});
			subscription = await api.get<SubscriptionStatusResponse>('/api/member/subscription');
			subMsg = 'Subscription resumed';
		} catch (err) {
			subMsg = err instanceof ApiError ? err.message : 'Could not resume subscription';
		} finally {
			resumeBusy = false;
		}
	}

	async function applyCoupon() {
		couponError = '';
		couponMsg = '';
		if (!couponCode.trim()) {
			couponError = 'Enter a coupon code';
			return;
		}
		couponBusy = true;
		try {
			const res = await api.post<CouponValidationResponse>('/api/member/coupons/apply', {
				code: couponCode.trim()
			});
			if (res.valid) {
				couponMsg = res.message || 'Coupon applied successfully';
				couponCode = '';
			} else {
				couponError = res.message || 'Invalid coupon code';
			}
		} catch (err) {
			couponError = err instanceof ApiError ? err.message : 'Failed to apply coupon';
		} finally {
			couponBusy = false;
		}
	}

	async function handleDelete() {
		if (deleteConfirm !== 'DELETE') {
			deleteError = 'Type DELETE to confirm';
			return;
		}
		deleteError = '';
		deleting = true;
		try {
			await api.del('/api/member/account');
			auth.logout();
			goto(resolve('/'));
		} catch (err) {
			deleteError = err instanceof ApiError ? err.message : 'Failed to delete account';
		} finally {
			deleting = false;
		}
	}

	async function saveNotificationPreferences() {
		notifSaving = true;
		notifMsg = '';
		notifError = '';
		try {
			await api.put('/api/member/notification-preferences', notifPrefs);
			notifMsg = 'Preferences saved';
			setTimeout(() => (notifMsg = ''), 3000);
		} catch (err) {
			notifError = err instanceof ApiError ? err.message : 'Failed to save preferences';
		} finally {
			notifSaving = false;
		}
	}
</script>

<svelte:head><title>Account - Precision Options Signals</title></svelte:head>

<div class="acct">
	<h2 class="acct-title">Account Settings</h2>

	<div class="tabs" role="tablist" aria-label="Account settings sections">
		<button
			type="button"
			role="tab"
			aria-selected={activeTab === 'profile'}
			aria-controls="tab-panel-profile"
			id="tab-profile"
			class="tab"
			class:tab--active={activeTab === 'profile'}
			onclick={() => selectTab('profile')}
		>
			<UserCircleIcon size={18} weight="duotone" />
			<span>Profile</span>
		</button>
		<button
			type="button"
			role="tab"
			aria-selected={activeTab === 'security'}
			aria-controls="tab-panel-security"
			id="tab-security"
			class="tab"
			class:tab--active={activeTab === 'security'}
			onclick={() => selectTab('security')}
		>
			<LockIcon size={18} weight="duotone" />
			<span>Security</span>
		</button>
		<button
			type="button"
			role="tab"
			aria-selected={activeTab === 'subscription'}
			aria-controls="tab-panel-subscription"
			id="tab-subscription"
			class="tab"
			class:tab--active={activeTab === 'subscription'}
			onclick={() => selectTab('subscription')}
		>
			<LightningIcon size={18} weight="duotone" />
			<span>Subscription</span>
		</button>
		<button
			type="button"
			role="tab"
			aria-selected={activeTab === 'notifications'}
			aria-controls="tab-panel-notifications"
			id="tab-notifications"
			class="tab"
			class:tab--active={activeTab === 'notifications'}
			onclick={() => selectTab('notifications')}
		>
			<BellIcon size={18} weight="duotone" />
			<span>Notifications</span>
		</button>
	</div>

	{#if activeTab === 'profile'}
		<div
			id="tab-panel-profile"
			role="tabpanel"
			aria-labelledby="tab-profile"
			class="card"
		>
			<form onsubmit={handleProfileSave} class="form">
				<div class="field">
					<label for="name" class="label">Display Name</label>
					<input id="name" type="text" bind:value={name} class="input" />
				</div>
				<div class="field">
					<span class="label">Email</span>
					<p class="static">{auth.user?.email}</p>
				</div>
				{#if profileError}<p class="err">{profileError}</p>{/if}
				<button type="submit" disabled={saving} class="btn-primary">
					{saving ? 'Saving...' : saved ? 'Saved!' : 'Save Changes'}
				</button>
			</form>
		</div>
	{/if}

	{#if activeTab === 'security'}
		<div
			id="tab-panel-security"
			role="tabpanel"
			aria-labelledby="tab-security"
			class="card"
		>
			<form onsubmit={handlePasswordChange} class="form">
				<div class="field">
					<label for="cur-pw" class="label">Current Password</label>
					<input
						id="cur-pw"
						type="password"
						bind:value={curPassword}
						class="input"
						autocomplete="current-password"
					/>
				</div>
				<div class="field">
					<label for="new-pw" class="label">New Password</label>
					<input
						id="new-pw"
						type="password"
						bind:value={newPassword}
						class="input"
						autocomplete="new-password"
					/>
				</div>
				<div class="field">
					<label for="confirm-pw" class="label">Confirm New Password</label>
					<input
						id="confirm-pw"
						type="password"
						bind:value={confirmPassword}
						class="input"
						autocomplete="new-password"
					/>
				</div>
				{#if pwError}<p class="err">{pwError}</p>{/if}
				{#if pwMsg}<p class="success">{pwMsg}</p>{/if}
				<button type="submit" disabled={pwSaving} class="btn-primary">
					{pwSaving ? 'Updating...' : 'Update Password'}
				</button>
			</form>

			<div class="divider">
				<span class="divider-line"></span>
				<span class="divider-label">Danger Zone</span>
				<span class="divider-line"></span>
			</div>

			<div class="danger-zone">
				<h3 class="danger-title">Delete Account</h3>
				<p class="muted">
					This action is permanent. All your data, progress, and subscription will be
					deleted.
				</p>
				<div class="delete-row">
					<input
						type="text"
						placeholder="Type &quot;DELETE&quot; to confirm"
						bind:value={deleteConfirm}
						class="input"
					/>
					<button
						type="button"
						class="btn-danger"
						disabled={deleting || deleteConfirm !== 'DELETE'}
						onclick={handleDelete}
					>
						{deleting ? 'Deleting...' : 'Delete Account'}
					</button>
				</div>
				{#if deleteError}<p class="err">{deleteError}</p>{/if}
			</div>
		</div>
	{/if}

	{#if activeTab === 'subscription'}
		<div
			id="tab-panel-subscription"
			role="tabpanel"
			aria-labelledby="tab-subscription"
			class="card"
		>
			{#if subscription?.subscription}
				{@const sub = subscription.subscription}
				<div class="sub-rows">
					<div class="sub-row">
						<span class="sub-label">Plan</span>
						<span class="sub-val">{planLabel(sub.plan)}</span>
					</div>
					<div class="sub-row">
						<span class="sub-label">Status</span>
						<span class="sub-status" style="color:{statusColor(sub.status)}"
							>{sub.status}</span
						>
					</div>
					<div class="sub-row">
						<span class="sub-label">Next Billing</span>
						<span class="sub-val">{formatDate(sub.current_period_end)}</span>
					</div>
				</div>
				<div class="sub-actions">
					<button
						type="button"
						class="btn-primary"
						disabled={billingBusy}
						onclick={openBilling}
					>
						{billingBusy ? 'Opening...' : 'Manage Billing'}
					</button>
					{#if sub.status === 'active' || sub.status === 'trialing'}
						<button
							type="button"
							class="btn-ghost"
							disabled={cancelBusy}
							onclick={cancelSubscription}
						>
							{cancelBusy ? 'Cancelling...' : 'Cancel Subscription'}
						</button>
					{/if}
					{#if sub.status === 'canceled'}
						<button
							type="button"
							class="btn-ghost"
							disabled={resumeBusy}
							onclick={resumeSubscription}
						>
							{resumeBusy ? 'Resuming...' : 'Resume Subscription'}
						</button>
					{/if}
				</div>
				{#if subMsg}<p class="info">{subMsg}</p>{/if}
			{:else}
				<p class="muted">No active subscription.</p>
				<a
					href={resolve('/pricing/monthly')}
					class="btn-primary btn-link"
				>
					View Plans
				</a>
			{/if}

			<div class="divider">
				<span class="divider-line"></span>
				<span class="divider-label">Coupon</span>
				<span class="divider-line"></span>
			</div>

			<div class="coupon-row">
				<input
					type="text"
					placeholder="Enter coupon code"
					bind:value={couponCode}
					class="input"
				/>
				<button
					type="button"
					class="btn-primary"
					disabled={couponBusy}
					onclick={applyCoupon}
				>
					{couponBusy ? 'Applying...' : 'Apply'}
				</button>
			</div>
			{#if couponError}<p class="err">{couponError}</p>{/if}
			{#if couponMsg}<p class="success">{couponMsg}</p>{/if}
		</div>
	{/if}

	{#if activeTab === 'notifications'}
		<div
			id="tab-panel-notifications"
			role="tabpanel"
			aria-labelledby="tab-notifications"
			class="card"
		>
			{#if notifLoading && !notifLoaded}
				<p class="muted">Loading preferences...</p>
			{:else}
				<div class="toggle-list">
					<label class="toggle">
						<input
							type="checkbox"
							bind:checked={notifPrefs.new_watchlist}
							class="toggle-input"
						/>
						<span class="toggle-text">
							<span class="toggle-title">New watchlist published</span>
							<span class="toggle-sub"
								>Get notified when a new watchlist is released.</span
							>
						</span>
					</label>
					<label class="toggle">
						<input
							type="checkbox"
							bind:checked={notifPrefs.new_course}
							class="toggle-input"
						/>
						<span class="toggle-text">
							<span class="toggle-title">New course added</span>
							<span class="toggle-sub"
								>Hear about freshly published courses and lessons.</span
							>
						</span>
					</label>
					<label class="toggle">
						<input
							type="checkbox"
							bind:checked={notifPrefs.marketing}
							class="toggle-input"
						/>
						<span class="toggle-text">
							<span class="toggle-title">Marketing &amp; promotions</span>
							<span class="toggle-sub"
								>Occasional product news, offers, and announcements.</span
							>
						</span>
					</label>
				</div>

				{#if notifError}<p class="err">{notifError}</p>{/if}
				{#if notifMsg}<p class="success">{notifMsg}</p>{/if}

				<button
					type="button"
					class="btn-primary"
					disabled={notifSaving}
					onclick={saveNotificationPreferences}
				>
					{notifSaving ? 'Saving...' : 'Save Preferences'}
				</button>
			{/if}
		</div>
	{/if}
</div>

<style>
	.acct-title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 1.5rem;
	}

	.tabs {
		display: flex;
		gap: 0.25rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		margin-bottom: 1.5rem;
		overflow-x: auto;
	}
	.tab {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.85rem 1rem;
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
		transition:
			color 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
		white-space: nowrap;
		margin-bottom: -1px;
	}
	.tab:hover {
		color: var(--color-white);
	}
	.tab--active {
		color: var(--color-white);
		border-bottom-color: var(--color-teal);
		font-weight: var(--w-semibold);
	}

	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.75rem;
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.label {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.input {
		padding: 0.65rem 0.85rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		transition: border-color 200ms var(--ease-out);
	}
	.input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.static {
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
	}

	.btn-primary {
		align-self: flex-start;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border: none;
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.btn-primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn-primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn-link {
		display: inline-block;
		margin-top: 0.75rem;
		text-decoration: none;
		text-align: center;
	}

	.btn-ghost {
		padding: 0.6rem 1.25rem;
		background: transparent;
		color: var(--color-white);
		font-weight: var(--w-medium);
		font-size: var(--fs-sm);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition:
			background 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}
	.btn-ghost:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.25);
	}
	.btn-ghost:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-danger {
		padding: 0.6rem 1.5rem;
		background: var(--color-red);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border: none;
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.btn-danger:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn-danger:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.err {
		color: #fca5a5;
		font-size: var(--fs-sm);
	}
	.success {
		color: var(--color-green);
		font-size: var(--fs-sm);
	}
	.info {
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		margin-top: 0.5rem;
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: var(--lh-relaxed);
	}

	.sub-rows {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.sub-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.sub-label {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
	}
	.sub-val {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}
	.sub-status {
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
		text-transform: capitalize;
	}
	.sub-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin-top: 1.25rem;
	}

	.coupon-row {
		display: flex;
		gap: 0.75rem;
		align-items: stretch;
	}
	.coupon-row .input {
		flex: 1;
	}

	.divider {
		display: flex;
		align-items: center;
		gap: 0.85rem;
		margin: 2rem 0 1.25rem;
	}
	.divider-line {
		flex: 1;
		height: 1px;
		background: rgba(255, 255, 255, 0.08);
	}
	.divider-label {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.danger-zone {
		border: 1px solid rgba(239, 68, 68, 0.2);
		background: rgba(239, 68, 68, 0.04);
		border-radius: var(--radius-lg);
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.danger-title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-red);
	}
	.delete-row {
		display: flex;
		gap: 0.75rem;
		align-items: stretch;
	}
	.delete-row .input {
		flex: 1;
	}

	.toggle-list {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
		margin-bottom: 1.5rem;
	}
	.toggle {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		padding: 1rem;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: border-color 200ms var(--ease-out);
	}
	.toggle:hover {
		border-color: rgba(255, 255, 255, 0.12);
	}
	.toggle-input {
		margin-top: 0.2rem;
		width: 1.05rem;
		height: 1.05rem;
		accent-color: var(--color-teal);
		cursor: pointer;
	}
	.toggle-text {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.toggle-title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}
	.toggle-sub {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}
</style>
