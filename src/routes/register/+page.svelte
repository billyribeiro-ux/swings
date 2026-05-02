<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AuthResponse } from '$lib/api/types';
	import { SITE } from '$lib/seo/config';
	import UserIcon from 'phosphor-svelte/lib/UserIcon';
	import EnvelopeSimpleIcon from 'phosphor-svelte/lib/EnvelopeSimpleIcon';
	import LockIcon from 'phosphor-svelte/lib/LockIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import EyeSlashIcon from 'phosphor-svelte/lib/EyeSlashIcon';
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';

	let name = $state('');
	let email = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let showPassword = $state(false);
	let showConfirm = $state(false);
	let error = $state('');
	let loading = $state(false);

	const passwordChecks = $derived({
		length: password.length >= 8,
		mixed: /[A-Za-z]/.test(password) && /[0-9]/.test(password),
		match: password.length > 0 && password === confirmPassword
	});

	const passwordStrength = $derived(
		[passwordChecks.length, passwordChecks.mixed, passwordChecks.match].filter(Boolean).length
	);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';

		if (password !== confirmPassword) {
			error = 'Passwords do not match';
			return;
		}

		if (password.length < 8) {
			error = 'Password must be at least 8 characters';
			return;
		}

		loading = true;

		try {
			const res = await api.post<AuthResponse>(
				'/api/auth/register',
				{ email, password, name },
				{ skipAuth: true }
			);

			// BFF (Phase 1.3): registration response includes Set-Cookie for
			// the new httpOnly session pair. We persist only the user record.
			auth.setUser(res.user);
			goto(resolve('/dashboard'));
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
	<title>Create account - {SITE.name}</title>
</svelte:head>

<div class="auth-page">
	<div class="auth-page__bg" aria-hidden="true"></div>

	<main class="auth-card">
		<header class="auth-card__header">
			<a href={resolve('/')} class="auth-card__logo" aria-label="{SITE.name} home">
				<span class="auth-card__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="auth-card__logo-accent">{SITE.logoBrandAccent}</span>
			</a>
			<h1 class="auth-card__title">Create your account</h1>
			<p class="auth-card__subtitle">Join thousands of traders · Takes less than a minute</p>
		</header>

		{#if error}
			<div class="auth-card__error" role="alert" aria-live="polite">{error}</div>
		{/if}

		<form onsubmit={handleSubmit} class="auth-form" novalidate>
			<div class="auth-form__field">
				<label for="name" class="auth-form__label">Full name</label>
				<div class="auth-form__control">
					<UserIcon size={18} weight="regular" class="auth-form__icon" />
					<input
						id="name"
						name="name"
						type="text"
						bind:value={name}
						required
						autocomplete="name"
						autocapitalize="words"
						class="auth-form__input auth-form__input--with-icon"
						placeholder="First and last name"
					/>
				</div>
			</div>

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
				<label for="password" class="auth-form__label">Password</label>
				<div class="auth-form__control">
					<LockIcon size={18} weight="regular" class="auth-form__icon" />
					<input
						id="password"
						name="password"
						type={showPassword ? 'text' : 'password'}
						bind:value={password}
						required
						autocomplete="new-password"
						class="auth-form__input auth-form__input--with-icon auth-form__input--with-action"
						placeholder="At least 8 characters"
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
				{#if password.length > 0}
					<div class="auth-form__strength" aria-hidden="true">
						<div class="auth-form__strength-bars">
							<span class={passwordStrength >= 1 ? 'on' : ''}></span>
							<span class={passwordStrength >= 2 ? 'on' : ''}></span>
							<span class={passwordStrength >= 3 ? 'on strong' : ''}></span>
						</div>
						<span class="auth-form__strength-label">
							{passwordStrength === 3
								? 'Strong'
								: passwordStrength === 2
									? 'Good'
									: 'Weak'}
						</span>
					</div>
				{/if}
			</div>

			<div class="auth-form__field">
				<label for="confirm" class="auth-form__label">Confirm password</label>
				<div class="auth-form__control">
					<LockIcon size={18} weight="regular" class="auth-form__icon" />
					<input
						id="confirm"
						name="confirm-password"
						type={showConfirm ? 'text' : 'password'}
						bind:value={confirmPassword}
						required
						autocomplete="new-password"
						class="auth-form__input auth-form__input--with-icon auth-form__input--with-action"
						placeholder="Repeat your password"
					/>
					<button
						type="button"
						class="auth-form__action"
						onclick={() => (showConfirm = !showConfirm)}
						aria-label={showConfirm ? 'Hide password' : 'Show password'}
					>
						{#if showConfirm}
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
					<span>Creating account…</span>
				{:else}
					<span>Create account</span>
					<ArrowRightIcon size={16} weight="bold" />
				{/if}
			</button>

			<p class="auth-form__legal">
				By creating an account, you agree to our
				<a href={resolve('/terms')} class="auth-card__link auth-card__link--inline">Terms</a
				>
				and
				<a href={resolve('/privacy')} class="auth-card__link auth-card__link--inline"
					>Privacy Policy</a
				>.
			</p>
		</form>

		<ul class="auth-card__perks" aria-label="What you get">
			<li>
				<CheckCircleIcon size={14} weight="fill" /> Verification link sent to your email
			</li>
			<li>
				<CheckCircleIcon size={14} weight="fill" /> Cancel anytime · No long-term commitment
			</li>
		</ul>

		<div class="auth-card__trust">
			<ShieldCheckIcon size={14} weight="fill" />
			<span>Encrypted connection · We never share your data</span>
		</div>

		<footer class="auth-card__footer">
			<p class="auth-card__footer-line">
				Already have an account?
				<a href={resolve('/login')} class="auth-card__link">Sign in</a>
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
		margin-bottom: 1.5rem;
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
		gap: 1rem;
	}

	.auth-form__field {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
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

	/* ── Password strength ─────────────────────────────────────────────── */
	.auth-form__strength {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		margin-top: 0.1rem;
	}

	.auth-form__strength-bars {
		display: flex;
		gap: 4px;
		flex: 1;
	}

	.auth-form__strength-bars span {
		flex: 1;
		height: 4px;
		border-radius: 2px;
		background-color: rgba(255, 255, 255, 0.08);
		transition: background-color 200ms var(--ease-out);
	}

	.auth-form__strength-bars span.on {
		background-color: rgba(234, 179, 8, 0.85);
	}

	.auth-form__strength-bars span.on.strong {
		background-color: var(--color-teal);
	}

	.auth-form__strength-label {
		color: var(--color-grey-400);
		font-size: 0.7rem;
		letter-spacing: 0.02em;
		min-width: 2.8rem;
		text-align: right;
	}

	/* ── Submit ─────────────────────────────────────────────────────────── */
	.auth-form__submit {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		width: 100%;
		height: 3rem;
		margin-top: 0.35rem;
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

	.auth-form__legal {
		margin: 0.5rem 0 0;
		color: var(--color-grey-500);
		font-size: 0.72rem;
		line-height: 1.5;
		text-align: center;
	}

	/* ── Perks strip ────────────────────────────────────────────────────── */
	.auth-card__perks {
		list-style: none;
		margin: 1.25rem 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.auth-card__perks li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
	}

	.auth-card__perks :global(svg) {
		color: var(--color-teal-light);
		flex-shrink: 0;
	}

	/* ── Trust strip ────────────────────────────────────────────────────── */
	.auth-card__trust {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		justify-content: center;
		width: 100%;
		margin-top: 1rem;
		color: var(--color-grey-500);
		font-size: 0.72rem;
		letter-spacing: 0.02em;
	}

	.auth-card__trust :global(svg) {
		color: var(--color-teal-light);
	}

	/* ── Footer ─────────────────────────────────────────────────────────── */
	.auth-card__footer {
		margin-top: 1rem;
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

	.auth-card__link {
		color: var(--color-teal-light);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: color 200ms var(--ease-out);
	}

	.auth-card__link--inline {
		font-weight: var(--w-medium);
	}

	.auth-card__link:hover {
		color: var(--color-white);
		text-decoration: underline;
	}
</style>
