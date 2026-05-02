<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import { auth } from '$lib/stores/auth.svelte';
	import type { UserResponse } from '$lib/api/types';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';

	let loading = $state(true);
	let loadError = $state('');

	let name = $state('');
	let email = $state('');
	let nameSaving = $state(false);
	let nameSuccess = $state('');
	let nameError = $state('');

	let currentPwd = $state('');
	let newPwd = $state('');
	let confirmPwd = $state('');
	let pwdBusy = $state(false);
	let pwdSuccess = $state('');
	let pwdError = $state('');

	let deleteConfirmText = $state('');
	let deleteBusy = $state(false);
	let deleteError = $state('');

	async function load() {
		loading = true;
		loadError = '';
		try {
			const me = await api.get<UserResponse>('/api/member/profile');
			name = me.name;
			email = me.email;
		} catch (e) {
			loadError = e instanceof ApiError ? e.message : 'Failed to load profile';
		} finally {
			loading = false;
		}
	}

	async function saveName(e: SubmitEvent) {
		e.preventDefault();
		nameSuccess = '';
		nameError = '';
		const trimmed = name.trim();
		if (!trimmed) {
			nameError = 'Name cannot be empty.';
			return;
		}
		nameSaving = true;
		try {
			const updated = await api.put<UserResponse>('/api/member/profile', { name: trimmed });
			name = updated.name;
			if (auth.user) {
				auth.setUser({ ...auth.user, name: updated.name });
			}
			nameSuccess = 'Profile updated.';
		} catch (err) {
			nameError = err instanceof ApiError ? err.message : 'Failed to save profile';
		} finally {
			nameSaving = false;
		}
	}

	async function changePassword(e: SubmitEvent) {
		e.preventDefault();
		pwdSuccess = '';
		pwdError = '';
		if (!currentPwd) {
			pwdError = 'Enter your current password.';
			return;
		}
		if (newPwd.length < 8) {
			pwdError = 'New password must be at least 8 characters.';
			return;
		}
		if (newPwd !== confirmPwd) {
			pwdError = 'New password and confirmation do not match.';
			return;
		}
		pwdBusy = true;
		try {
			await api.post<{ ok: boolean }>('/api/member/password', {
				current_password: currentPwd,
				new_password: newPwd
			});
			currentPwd = '';
			newPwd = '';
			confirmPwd = '';
			pwdSuccess = 'Password updated. Other sessions have been signed out.';
		} catch (err) {
			pwdError = err instanceof ApiError ? err.message : 'Failed to change password';
		} finally {
			pwdBusy = false;
		}
	}

	async function deleteAccount() {
		deleteError = '';
		if (deleteConfirmText !== 'DELETE') {
			deleteError = 'Type DELETE to confirm account deletion.';
			return;
		}
		deleteBusy = true;
		try {
			await api.del('/api/member/account');
			await auth.logout();
			await goto(resolve('/'));
		} catch (err) {
			deleteError = err instanceof ApiError ? err.message : 'Failed to delete account';
			deleteBusy = false;
		}
	}

	onMount(() => {
		void load();
	});
</script>

<svelte:head><title>Account Details - Precision Options Signals</title></svelte:head>

<section class="ad">
	<header class="ad__header">
		<h1 class="ad__title">Account Details</h1>
		<p class="ad__sub">Manage your profile, password, and account.</p>
	</header>

	{#if loading}
		<p class="ad__muted">Loading…</p>
	{:else if loadError}
		<div class="ad__error" role="alert">{loadError}</div>
	{:else}
		<!-- Profile -->
		<form class="ad__card" onsubmit={saveName} novalidate>
			<h2 class="ad__section-title">Profile</h2>
			<div class="ad__field">
				<label class="ad__label" for="ad-name">Name</label>
				<input
					id="ad-name"
					class="ad__input"
					type="text"
					autocomplete="name"
					bind:value={name}
					disabled={nameSaving}
				/>
			</div>
			<div class="ad__field">
				<label class="ad__label" for="ad-email">Email</label>
				<input
					id="ad-email"
					class="ad__input"
					type="email"
					value={email}
					readonly
					aria-readonly="true"
				/>
				<span class="ad__hint">Contact support to change your email address.</span>
			</div>

			{#if nameSuccess}
				<p class="ad__success" role="status">
					<CheckCircleIcon size={14} weight="fill" />
					{nameSuccess}
				</p>
			{/if}
			{#if nameError}
				<p class="ad__inline-error" role="alert">
					<XCircleIcon size={14} weight="fill" />
					{nameError}
				</p>
			{/if}

			<div class="ad__actions">
				<button type="submit" class="btn btn--primary" disabled={nameSaving}>
					{nameSaving ? 'Saving…' : 'Save changes'}
				</button>
			</div>
		</form>

		<!-- Change Password -->
		<form class="ad__card" onsubmit={changePassword} novalidate>
			<h2 class="ad__section-title">Change password</h2>
			<div class="ad__field">
				<label class="ad__label" for="ad-current-pwd">Current password</label>
				<input
					id="ad-current-pwd"
					class="ad__input"
					type="password"
					autocomplete="current-password"
					bind:value={currentPwd}
					disabled={pwdBusy}
				/>
			</div>
			<div class="ad__field">
				<label class="ad__label" for="ad-new-pwd">New password</label>
				<input
					id="ad-new-pwd"
					class="ad__input"
					type="password"
					autocomplete="new-password"
					minlength="8"
					bind:value={newPwd}
					disabled={pwdBusy}
				/>
				<span class="ad__hint">At least 8 characters.</span>
			</div>
			<div class="ad__field">
				<label class="ad__label" for="ad-confirm-pwd">Confirm new password</label>
				<input
					id="ad-confirm-pwd"
					class="ad__input"
					type="password"
					autocomplete="new-password"
					minlength="8"
					bind:value={confirmPwd}
					disabled={pwdBusy}
				/>
			</div>

			{#if pwdSuccess}
				<p class="ad__success" role="status">
					<CheckCircleIcon size={14} weight="fill" />
					{pwdSuccess}
				</p>
			{/if}
			{#if pwdError}
				<p class="ad__inline-error" role="alert">
					<XCircleIcon size={14} weight="fill" />
					{pwdError}
				</p>
			{/if}

			<div class="ad__actions">
				<button type="submit" class="btn btn--primary" disabled={pwdBusy}>
					{pwdBusy ? 'Updating…' : 'Update password'}
				</button>
			</div>
		</form>

		<!-- Danger Zone -->
		<section class="ad__card ad__card--danger" aria-labelledby="danger-title">
			<div class="ad__danger-head">
				<WarningIcon size={20} weight="fill" />
				<h2 id="danger-title" class="ad__section-title">Danger zone</h2>
			</div>
			<p class="ad__copy">
				Deleting your account is permanent. We'll cancel any active subscription and erase
				your profile. This cannot be undone.
			</p>
			<div class="ad__field">
				<label class="ad__label" for="ad-delete-confirm">
					Type <strong>DELETE</strong> to confirm
				</label>
				<input
					id="ad-delete-confirm"
					class="ad__input"
					type="text"
					autocomplete="off"
					bind:value={deleteConfirmText}
					disabled={deleteBusy}
					placeholder="DELETE"
				/>
			</div>

			{#if deleteError}
				<p class="ad__inline-error" role="alert">
					<XCircleIcon size={14} weight="fill" />
					{deleteError}
				</p>
			{/if}

			<div class="ad__actions">
				<button
					type="button"
					class="btn btn--danger"
					onclick={deleteAccount}
					disabled={deleteBusy || deleteConfirmText !== 'DELETE'}
				>
					{deleteBusy ? 'Deleting…' : 'Delete account'}
				</button>
			</div>
		</section>
	{/if}
</section>

<style>
	.ad {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.ad__header {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.ad__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.ad__sub {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.ad__muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.ad__error {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}

	.ad__card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
		max-width: 40rem;
	}
	.ad__card--danger {
		border-color: rgba(224, 72, 72, 0.25);
		border-top: 2px solid rgba(224, 72, 72, 0.55);
	}

	.ad__section-title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.ad__danger-head {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-red);
	}
	.ad__copy {
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		line-height: 1.55;
	}

	.ad__field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.ad__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.ad__hint {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.ad__input {
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		font-size: var(--fs-sm);
		width: 100%;
	}
	.ad__input:focus {
		outline: none;
		border-color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.06);
	}
	.ad__input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.ad__input[readonly] {
		opacity: 0.7;
		cursor: not-allowed;
		background-color: rgba(255, 255, 255, 0.02);
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.65rem 1.1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		border: 1px solid transparent;
		transition:
			opacity 200ms var(--ease-out),
			background-color 200ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
	}
	.btn--primary:not(:disabled):hover {
		opacity: 0.9;
	}
	.btn--danger {
		background-color: rgba(224, 72, 72, 0.12);
		border-color: rgba(224, 72, 72, 0.35);
		color: var(--color-red);
	}
	.btn--danger:not(:disabled):hover {
		background-color: rgba(224, 72, 72, 0.22);
	}

	.ad__actions {
		display: flex;
		justify-content: flex-end;
		padding-top: 0.5rem;
	}
	.ad__success {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(34, 181, 115, 0.1);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		font-size: var(--fs-sm);
	}
	.ad__inline-error {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}

	@media (max-width: 640px) {
		.ad__card {
			padding: 1rem;
		}
	}
</style>
