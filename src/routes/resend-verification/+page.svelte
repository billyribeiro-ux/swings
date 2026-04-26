<script lang="ts">
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import { SITE } from '$lib/seo/config';

	let email = $state('');
	let loading = $state(false);
	let error = $state('');
	let success = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		try {
			await api.post('/api/auth/resend-verification', { email }, { skipAuth: true });
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
	<title>Resend Verification - {SITE.name}</title>
</svelte:head>

<div class="verify-page">
	<div class="verify-card">
		<a href={resolve('/')} class="verify-card__logo">
			<span class="verify-card__logo-brand">{SITE.logoBrandPrimary}</span>
			<span class="verify-card__logo-accent">{SITE.logoBrandAccent}</span>
		</a>
		<h1 class="verify-card__title">Resend Verification Email</h1>
		<p class="verify-card__subtitle">
			Enter your account email to receive a fresh verification link.
		</p>

		{#if success}
			<div class="verify-card__success">
				If the account exists and is not verified yet, a new verification email has been
				sent.
			</div>
			<a href={resolve('/login')} class="verify-card__back">Back to sign in</a>
		{:else}
			{#if error}
				<div class="verify-card__error">{error}</div>
			{/if}
			<form onsubmit={handleSubmit} class="verify-form">
				<label for="verify-email" class="verify-form__label">Email</label>
				<input
					id="verify-email"
					name="email"
					type="email"
					bind:value={email}
					required
					autocomplete="email"
					class="verify-form__input"
					placeholder="you@example.com"
				/>
				<button type="submit" disabled={loading} class="verify-form__submit">
					{loading ? 'Sending...' : 'Resend Verification'}
				</button>
			</form>
			<a href={resolve('/login')} class="verify-card__back">Back to sign in</a>
		{/if}
	</div>
</div>

<style>
	.verify-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		background: linear-gradient(145deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
	}

	.verify-card {
		width: 100%;
		max-width: 26rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 2.5rem;
	}

	.verify-card__logo {
		display: inline-flex;
		gap: 0.35rem;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 0.75rem;
	}

	.verify-card__logo-brand {
		color: var(--color-white);
	}
	.verify-card__logo-accent {
		color: var(--color-teal);
	}

	.verify-card__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.verify-card__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}

	.verify-card__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
		text-align: center;
	}

	.verify-card__success {
		background-color: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.3);
		color: #86efac;
		padding: 1rem 1.25rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		line-height: 1.5;
		margin-bottom: 1rem;
	}

	.verify-form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.verify-form__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.verify-form__input {
		width: 100%;
		padding: 0.75rem 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
	}

	.verify-form__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.verify-form__submit {
		width: 100%;
		padding: 0.85rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
	}

	.verify-form__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.verify-card__back {
		display: block;
		text-align: center;
		margin-top: 1.25rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
	}

	.verify-card__back:hover {
		color: var(--color-teal);
	}
</style>
