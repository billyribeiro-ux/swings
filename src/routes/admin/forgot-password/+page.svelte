<script lang="ts">
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
			await api.post('/api/auth/forgot-password', { email }, { skipAuth: true });
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
	<title>Forgot Password - Admin - {SITE.name}</title>
</svelte:head>

<div class="forgot-page">
	<div class="forgot-card">
		<div class="forgot-card__header">
			<a href="/" class="forgot-card__logo">
				<span class="forgot-card__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="forgot-card__logo-accent">{SITE.logoBrandAccent}</span>
			</a>
			<span class="forgot-card__badge">Admin</span>
			<h1 class="forgot-card__title">Forgot Password</h1>
			<p class="forgot-card__subtitle">
				Enter your email and we'll send you a link to reset your password
			</p>
		</div>

		{#if success}
			<div class="forgot-card__success">
				<p>If an account with that email exists, a password reset link has been sent.</p>
				<p class="forgot-card__success-hint">Check your email inbox and spam folder.</p>
			</div>
			<a href="/admin" class="forgot-card__back-btn">← Back to login</a>
		{:else}
			{#if error}
				<div class="forgot-card__error">{error}</div>
			{/if}

			<form onsubmit={handleSubmit} class="forgot-form">
				<div class="forgot-form__field">
					<label for="reset-email" class="forgot-form__label">Email</label>
					<input
						id="reset-email"
						name="email"
						type="email"
						bind:value={email}
						required
						autocomplete="email"
						class="forgot-form__input"
						placeholder="you@example.com"
					/>
				</div>

				<button type="submit" disabled={loading} class="forgot-form__submit">
					{loading ? 'Sending...' : 'Send Reset Link'}
				</button>
			</form>

			<a href="/admin" class="forgot-card__back">← Back to login</a>
		{/if}
	</div>
</div>

<style>
	.forgot-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		background: linear-gradient(145deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
	}

	.forgot-card {
		width: 100%;
		max-width: 26rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 2.5rem;
	}

	.forgot-card__header {
		text-align: center;
		margin-bottom: 2rem;
	}

	.forgot-card__logo {
		display: inline-flex;
		gap: 0.35rem;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 0.75rem;
	}

	.forgot-card__logo-brand {
		color: var(--color-white);
	}
	.forgot-card__logo-accent {
		color: var(--color-teal);
	}

	.forgot-card__badge {
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

	.forgot-card__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.forgot-card__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: 1.5;
	}

	.forgot-card__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
		text-align: center;
	}

	.forgot-card__success {
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

	.forgot-card__success-hint {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		margin-top: 0.5rem;
	}

	.forgot-form {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.forgot-form__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.forgot-form__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.forgot-form__input {
		width: 100%;
		padding: 0.75rem 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
		transition: border-color 200ms var(--ease-out);
	}

	.forgot-form__input::placeholder {
		color: var(--color-grey-500);
	}
	.forgot-form__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.forgot-form__submit {
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
	}

	.forgot-form__submit:hover:not(:disabled) {
		opacity: 0.9;
		transform: translateY(-1px);
	}
	.forgot-form__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.forgot-card__back,
	.forgot-card__back-btn {
		display: block;
		text-align: center;
		margin-top: 1.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		transition: color 200ms;
	}

	.forgot-card__back:hover,
	.forgot-card__back-btn:hover {
		color: var(--color-teal);
	}
</style>
