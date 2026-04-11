<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { api, ApiError } from '$lib/api/client';

	const token = $derived(page.url.searchParams.get('token') || '');

	let newPassword = $state('');
	let confirmPassword = $state('');
	let loading = $state(false);
	let error = $state('');
	let success = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';

		if (!token) {
			error = 'Invalid reset link. Please request a new one.';
			return;
		}

		if (newPassword.length < 8) {
			error = 'Password must be at least 8 characters.';
			return;
		}

		if (newPassword !== confirmPassword) {
			error = 'Passwords do not match.';
			return;
		}

		loading = true;

		try {
			await api.post(
				'/api/auth/reset-password',
				{ token, new_password: newPassword },
				{ skipAuth: true }
			);
			success = true;
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.message;
			} else {
				error = 'Something went wrong. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Reset Password - Admin - Explosive Swings</title>
</svelte:head>

<div class="reset-page">
	<div class="reset-card">
		<div class="reset-card__header">
			<a href="/" class="reset-card__logo">
				<span class="reset-card__logo-brand">Explosive</span>
				<span class="reset-card__logo-accent">Swings</span>
			</a>
			<span class="reset-card__badge">Admin</span>
			<h1 class="reset-card__title">Reset Password</h1>
			<p class="reset-card__subtitle">Enter your new password below</p>
		</div>

		{#if !token}
			<div class="reset-card__error">Invalid reset link. Please request a new password reset.</div>
			<a href="/admin/forgot-password" class="reset-card__back-btn">Request new reset link</a>
		{:else if success}
			<div class="reset-card__success">
				<p>Your password has been reset successfully!</p>
			</div>
			<button onclick={() => goto('/admin')} class="reset-card__login-btn"> Go to Login </button>
		{:else}
			{#if error}
				<div class="reset-card__error">{error}</div>
			{/if}

			<form onsubmit={handleSubmit} class="reset-form">
				<div class="reset-form__field">
					<label for="new-password" class="reset-form__label">New Password</label>
					<input
						id="new-password"
						name="new-password"
						type="password"
						bind:value={newPassword}
						required
						minlength={8}
						autocomplete="new-password"
						class="reset-form__input"
						placeholder="At least 8 characters"
					/>
				</div>

				<div class="reset-form__field">
					<label for="confirm-password" class="reset-form__label">Confirm Password</label>
					<input
						id="confirm-password"
						name="confirm-password"
						type="password"
						bind:value={confirmPassword}
						required
						minlength={8}
						autocomplete="new-password"
						class="reset-form__input"
						placeholder="Repeat your password"
					/>
				</div>

				<button type="submit" disabled={loading} class="reset-form__submit">
					{loading ? 'Resetting...' : 'Reset Password'}
				</button>
			</form>

			<a href="/admin" class="reset-card__back">← Back to login</a>
		{/if}
	</div>
</div>

<style>
	.reset-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		background: linear-gradient(145deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
	}

	.reset-card {
		width: 100%;
		max-width: 26rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 2.5rem;
	}

	.reset-card__header {
		text-align: center;
		margin-bottom: 2rem;
	}

	.reset-card__logo {
		display: inline-flex;
		gap: 0.35rem;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 0.75rem;
	}

	.reset-card__logo-brand {
		color: var(--color-white);
	}
	.reset-card__logo-accent {
		color: var(--color-teal);
	}

	.reset-card__badge {
		display: inline-block;
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: #f59e0b;
		background-color: rgba(245, 158, 11, 0.12);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 1rem;
	}

	.reset-card__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.reset-card__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.reset-card__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
		text-align: center;
	}

	.reset-card__success {
		background-color: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.3);
		color: #86efac;
		padding: 1rem 1.25rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		text-align: center;
		margin-bottom: 1.5rem;
		line-height: 1.5;
	}

	.reset-form {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.reset-form__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.reset-form__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.reset-form__input {
		width: 100%;
		padding: 0.75rem 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
		transition: border-color 200ms var(--ease-out);
	}

	.reset-form__input::placeholder {
		color: var(--color-grey-500);
	}
	.reset-form__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.reset-form__submit,
	.reset-card__login-btn {
		width: 100%;
		padding: 0.85rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		transition:
			opacity 200ms var(--ease-out),
			transform 200ms var(--ease-out);
		text-align: center;
		text-decoration: none;
		display: block;
	}

	.reset-form__submit:hover:not(:disabled),
	.reset-card__login-btn:hover {
		opacity: 0.9;
		transform: translateY(-1px);
	}
	.reset-form__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.reset-card__back,
	.reset-card__back-btn {
		display: block;
		text-align: center;
		margin-top: 1.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		transition: color 200ms;
	}

	.reset-card__back:hover,
	.reset-card__back-btn:hover {
		color: var(--color-teal);
	}
</style>
