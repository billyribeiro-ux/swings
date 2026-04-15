<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AuthResponse } from '$lib/api/types';
	import { SITE } from '$lib/seo/config';

	let email = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		try {
			const res = await api.post<AuthResponse>(
				'/api/auth/login',
				{ email, password },
				{ skipAuth: true }
			);

			auth.setAuth(res.user, res.access_token, res.refresh_token);

			if (res.user.role.toLowerCase() === 'admin') {
				goto('/admin');
			} else {
				goto('/dashboard');
			}
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.status === 401 ? 'Invalid email or password' : err.message;
			} else {
				error = 'Something went wrong. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Login - {SITE.name}</title>
</svelte:head>

<div class="auth-page">
	<div class="auth-card">
		<div class="auth-card__header">
			<a href="/" class="auth-card__logo">
				<span class="auth-card__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="auth-card__logo-accent">{SITE.logoBrandAccent}</span>
			</a>
			<h1 class="auth-card__title">Welcome Back</h1>
			<p class="auth-card__subtitle">Sign in to access your dashboard</p>
		</div>

		{#if error}
			<div class="auth-card__error">{error}</div>
		{/if}

		<form onsubmit={handleSubmit} class="auth-form">
			<div class="auth-form__field">
				<label for="email" class="auth-form__label">Email</label>
				<input
					id="email"
					name="email"
					type="email"
					bind:value={email}
					required
					autocomplete="email"
					class="auth-form__input"
					placeholder="you@example.com"
				/>
			</div>

			<div class="auth-form__field">
				<label for="password" class="auth-form__label">Password</label>
				<input
					id="password"
					name="password"
					type="password"
					bind:value={password}
					required
					autocomplete="current-password"
					class="auth-form__input"
					placeholder="Enter your password"
				/>
			</div>

			<button type="submit" disabled={loading} class="auth-form__submit">
				{loading ? 'Signing in...' : 'Sign In'}
			</button>
		</form>

		<p class="auth-card__footer">
			Don't have an account?
			<a href="/register" class="auth-card__link">Create one</a>
		</p>
	</div>
</div>

<style>
	.auth-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
		background: linear-gradient(145deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
	}

	.auth-card {
		width: 100%;
		max-width: 26rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 2.5rem;
	}

	.auth-card__header {
		text-align: center;
		margin-bottom: 2rem;
	}

	.auth-card__logo {
		display: inline-flex;
		gap: 0.35rem;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 1.5rem;
	}

	.auth-card__logo-brand {
		color: var(--color-white);
	}

	.auth-card__logo-accent {
		color: var(--color-teal);
	}

	.auth-card__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.auth-card__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.auth-card__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
		text-align: center;
	}

	.auth-form {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.auth-form__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.auth-form__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.auth-form__input {
		width: 100%;
		padding: 0.75rem 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
		transition: border-color 200ms var(--ease-out);
	}

	.auth-form__input::placeholder {
		color: var(--color-grey-500);
	}

	.auth-form__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.auth-form__submit {
		width: 100%;
		padding: 0.85rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		border-radius: var(--radius-lg);
		transition:
			opacity 200ms var(--ease-out),
			transform 200ms var(--ease-out);
		cursor: pointer;
	}

	.auth-form__submit:hover:not(:disabled) {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	.auth-form__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.auth-card__footer {
		text-align: center;
		margin-top: 1.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.auth-card__link {
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	.auth-card__link:hover {
		text-decoration: underline;
	}
</style>
