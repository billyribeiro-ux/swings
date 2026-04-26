<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AuthResponse } from '$lib/api/types';
	import { SITE } from '$lib/seo/config';
	import EnvelopeSimpleIcon from 'phosphor-svelte/lib/EnvelopeSimpleIcon';
	import LockIcon from 'phosphor-svelte/lib/LockIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import EyeSlashIcon from 'phosphor-svelte/lib/EyeSlashIcon';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';

	let email = $state('');
	let password = $state('');
	let showPassword = $state(false);
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

			// BFF (Phase 1.3): the access + refresh tokens were just landed
			// as httpOnly cookies by the server's `Set-Cookie` headers — JS
			// neither sees them nor needs them. We persist only the
			// non-sensitive `user` record so the UI shell renders without a
			// flash before `/api/auth/me` round-trips on the next page.
			auth.setUser(res.user);

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
	<title>Sign in - {SITE.name}</title>
</svelte:head>

<div class="auth-page">
	<div class="auth-page__bg" aria-hidden="true"></div>

	<main class="auth-card">
		<header class="auth-card__header">
			<a href="/" class="auth-card__logo" aria-label="{SITE.name} home">
				<span class="auth-card__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="auth-card__logo-accent">{SITE.logoBrandAccent}</span>
			</a>
			<h1 class="auth-card__title">Welcome back</h1>
			<p class="auth-card__subtitle">Sign in to access your dashboard</p>
		</header>

		{#if error}
			<div class="auth-card__error" role="alert" aria-live="polite">{error}</div>
		{/if}

		<form onsubmit={handleSubmit} class="auth-form" novalidate>
			<div class="auth-form__field">
				<label for="email" class="auth-form__label">Email</label>
				<div class="auth-form__control">
					<EnvelopeSimpleIcon size={18} weight="regular" class="auth-form__icon" />
					<input
						id="email"
						name="email"
						type="email"
						bind:value={email}
						required
						autocomplete="email"
						inputmode="email"
						autocapitalize="off"
						spellcheck="false"
						class="auth-form__input auth-form__input--with-icon"
						placeholder="you@example.com"
					/>
				</div>
			</div>

			<div class="auth-form__field">
				<div class="auth-form__label-row">
					<label for="password" class="auth-form__label">Password</label>
					<a href="/forgot-password" class="auth-form__forgot">Forgot password?</a>
				</div>
				<div class="auth-form__control">
					<LockIcon size={18} weight="regular" class="auth-form__icon" />
					<input
						id="password"
						name="password"
						type={showPassword ? 'text' : 'password'}
						bind:value={password}
						required
						autocomplete="current-password"
						class="auth-form__input auth-form__input--with-icon auth-form__input--with-action"
						placeholder="Enter your password"
					/>
					<button
						type="button"
						class="auth-form__action"
						onclick={() => (showPassword = !showPassword)}
						aria-label={showPassword ? 'Hide password' : 'Show password'}
					>
						{#if showPassword}
							<EyeSlashIcon size={18} weight="regular" />
						{:else}
							<EyeIcon size={18} weight="regular" />
						{/if}
					</button>
				</div>
			</div>

			<button type="submit" disabled={loading} class="auth-form__submit">
				{#if loading}
					<span class="auth-form__spinner" aria-hidden="true"></span>
					<span>Signing in…</span>
				{:else}
					<span>Sign in</span>
					<ArrowRightIcon size={16} weight="bold" />
				{/if}
			</button>
		</form>

		<div class="auth-card__trust">
			<ShieldCheckIcon size={14} weight="fill" />
			<span>Encrypted connection · We never share your data</span>
		</div>

		<footer class="auth-card__footer">
			<p class="auth-card__footer-line">
				Don't have an account?
				<a href="/register" class="auth-card__link">Create one</a>
			</p>
			<p class="auth-card__footer-line auth-card__footer-line--muted">
				Need a new verification link?
				<a href="/resend-verification" class="auth-card__link">Resend email</a>
			</p>
		</footer>
	</main>
</div>

<style>
	/* ── Page shell ─────────────────────────────────────────────────────── */
	.auth-page {
		position: relative;
		min-height: 100vh;
		display: grid;
		place-items: center;
		padding: clamp(1rem, 4vw, 3rem);
		background:
			radial-gradient(
				ellipse 120% 80% at 50% -10%,
				rgba(15, 164, 175, 0.18),
				transparent 60%
			),
			linear-gradient(180deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
		isolation: isolate;
		overflow: hidden;
	}

	.auth-page__bg {
		position: absolute;
		inset: 0;
		background-image:
			linear-gradient(to right, rgba(255, 255, 255, 0.04) 1px, transparent 1px),
			linear-gradient(to bottom, rgba(255, 255, 255, 0.04) 1px, transparent 1px);
		background-size: 48px 48px;
		mask-image: radial-gradient(ellipse 80% 70% at 50% 30%, black 0%, transparent 75%);
		z-index: -1;
		opacity: 0.6;
	}

	/* ── Card ───────────────────────────────────────────────────────────── */
	.auth-card {
		width: 100%;
		max-width: 28rem;
		background: linear-gradient(165deg, rgba(19, 43, 80, 0.95) 0%, rgba(12, 27, 46, 0.95) 100%);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: clamp(1.75rem, 3vw, 2.5rem);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.04) inset,
			0 24px 64px -24px rgba(0, 0, 0, 0.6),
			0 0 0 1px rgba(15, 164, 175, 0.05);
		backdrop-filter: blur(20px) saturate(1.2);
	}

	/* ── Header ─────────────────────────────────────────────────────────── */
	.auth-card__header {
		text-align: center;
		margin-bottom: 1.75rem;
	}

	.auth-card__logo {
		display: inline-flex;
		align-items: baseline;
		gap: 0.4rem;
		font-size: clamp(1.125rem, 2.2vw, 1.25rem);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		letter-spacing: -0.02em;
		text-decoration: none;
		margin-bottom: 1.75rem;
		white-space: nowrap;
		transition: transform 200ms var(--ease-out);
	}

	.auth-card__logo:hover {
		transform: translateY(-1px);
	}

	.auth-card__logo-brand {
		color: var(--color-white);
	}

	.auth-card__logo-accent {
		background: linear-gradient(135deg, var(--color-teal-light), var(--color-teal));
		-webkit-background-clip: text;
		background-clip: text;
		color: transparent;
	}

	.auth-card__title {
		font-size: clamp(1.6rem, 3vw, 1.875rem);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		letter-spacing: -0.02em;
		line-height: 1.15;
		margin: 0 0 0.5rem;
	}

	.auth-card__subtitle {
		margin: 0;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: 1.5;
	}

	/* ── Error banner ───────────────────────────────────────────────────── */
	.auth-card__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.35);
		color: #fecaca;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1.25rem;
		text-align: center;
	}

	/* ── Form ───────────────────────────────────────────────────────────── */
	.auth-form {
		display: flex;
		flex-direction: column;
		gap: 1.1rem;
	}

	.auth-form__field {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
	}

	.auth-form__label-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}

	.auth-form__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-200);
		letter-spacing: 0.01em;
	}

	.auth-form__control {
		position: relative;
		display: flex;
		align-items: center;
	}

	:global(.auth-form__icon) {
		position: absolute;
		left: 0.9rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
		transition: color 200ms var(--ease-out);
	}

	.auth-form__control:focus-within :global(.auth-form__icon) {
		color: var(--color-teal-light);
	}

	.auth-form__input {
		width: 100%;
		height: 2.875rem;
		padding: 0 1rem;
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
		font-family: inherit;
		transition:
			border-color 200ms var(--ease-out),
			background-color 200ms var(--ease-out),
			box-shadow 200ms var(--ease-out);
	}

	.auth-form__input--with-icon {
		padding-left: 2.75rem;
	}

	.auth-form__input--with-action {
		padding-right: 2.75rem;
	}

	.auth-form__input::placeholder {
		color: var(--color-grey-500);
	}

	.auth-form__input:hover:not(:focus) {
		border-color: rgba(255, 255, 255, 0.16);
	}

	.auth-form__input:focus {
		outline: none;
		border-color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.06);
		box-shadow: 0 0 0 4px rgba(15, 164, 175, 0.15);
	}

	.auth-form__input:autofill {
		box-shadow: 0 0 0 1000px rgba(15, 164, 175, 0.08) inset;
		-webkit-box-shadow: 0 0 0 1000px rgba(15, 164, 175, 0.08) inset;
		-webkit-text-fill-color: var(--color-white);
		caret-color: var(--color-white);
	}

	.auth-form__action {
		position: absolute;
		right: 0.55rem;
		top: 50%;
		transform: translateY(-50%);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-400);
		cursor: pointer;
		transition:
			color 200ms var(--ease-out),
			background-color 200ms var(--ease-out);
	}

	.auth-form__action:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.06);
	}

	.auth-form__action:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 1px;
	}

	.auth-form__forgot {
		font-size: var(--fs-xs);
		color: var(--color-teal-light);
		text-decoration: none;
		font-weight: var(--w-medium);
		transition: color 200ms var(--ease-out);
	}

	.auth-form__forgot:hover {
		color: var(--color-white);
		text-decoration: underline;
	}

	/* ── Submit ─────────────────────────────────────────────────────────── */
	.auth-form__submit {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		width: 100%;
		height: 3rem;
		margin-top: 0.5rem;
		background: linear-gradient(135deg, var(--color-teal-light), var(--color-teal));
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		letter-spacing: 0.01em;
		border: none;
		border-radius: var(--radius-lg);
		cursor: pointer;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.15) inset,
			0 8px 24px -12px rgba(15, 164, 175, 0.6);
		transition:
			transform 200ms var(--ease-out),
			box-shadow 200ms var(--ease-out),
			filter 200ms var(--ease-out);
	}

	.auth-form__submit:hover:not(:disabled) {
		transform: translateY(-1px);
		filter: brightness(1.05);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.15) inset,
			0 12px 32px -12px rgba(15, 164, 175, 0.75);
	}

	.auth-form__submit:active:not(:disabled) {
		transform: translateY(0);
	}

	.auth-form__submit:disabled {
		opacity: 0.65;
		cursor: not-allowed;
	}

	.auth-form__submit:focus-visible {
		outline: 2px solid var(--color-white);
		outline-offset: 3px;
	}

	.auth-form__spinner {
		width: 1rem;
		height: 1rem;
		border-radius: 50%;
		border: 2px solid rgba(255, 255, 255, 0.35);
		border-top-color: var(--color-white);
		animation: auth-spin 0.7s linear infinite;
	}

	@keyframes auth-spin {
		to {
			transform: rotate(360deg);
		}
	}

	/* ── Trust strip ────────────────────────────────────────────────────── */
	.auth-card__trust {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		justify-content: center;
		width: 100%;
		margin-top: 1.25rem;
		color: var(--color-grey-500);
		font-size: 0.72rem;
		letter-spacing: 0.02em;
	}

	.auth-card__trust :global(svg) {
		color: var(--color-teal-light);
	}

	/* ── Footer ─────────────────────────────────────────────────────────── */
	.auth-card__footer {
		margin-top: 1.25rem;
		padding-top: 1.25rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		text-align: center;
	}

	.auth-card__footer-line {
		margin: 0;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: 1.5;
	}

	.auth-card__footer-line--muted {
		margin-top: 0.4rem;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
	}

	.auth-card__link {
		color: var(--color-teal-light);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: color 200ms var(--ease-out);
	}

	.auth-card__link:hover {
		color: var(--color-white);
		text-decoration: underline;
	}
</style>
