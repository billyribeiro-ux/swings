<script lang="ts">
	import { page } from '$app/state';
	import { api, ApiError } from '$lib/api/client';
	import { SITE } from '$lib/seo/config';
	import { onMount } from 'svelte';

	const token = $derived(page.url.searchParams.get('token') || '');

	let verifying = $state(true);
	let success = $state(false);
	let message = $state('');
	let error = $state('');

	async function verify() {
		if (!token) {
			verifying = false;
			error = 'Invalid verification link. Please request a new one.';
			return;
		}
		verifying = true;
		error = '';
		try {
			const res = await api.post<{ message?: string }>('/api/auth/verify-email', { token }, { skipAuth: true });
			success = true;
			message = res.message ?? 'Email verified successfully.';
		} catch (err) {
			success = false;
			if (err instanceof ApiError) {
				error = err.message;
			} else {
				error = 'Unable to verify email. Please try again.';
			}
		} finally {
			verifying = false;
		}
	}

	onMount(() => {
		void verify();
	});
</script>

<svelte:head>
	<title>Verify Email - {SITE.name}</title>
</svelte:head>

<div class="verify-page">
	<div class="verify-card">
		<a href="/" class="verify-card__logo">
			<span class="verify-card__logo-brand">{SITE.logoBrandPrimary}</span>
			<span class="verify-card__logo-accent">{SITE.logoBrandAccent}</span>
		</a>
		<h1 class="verify-card__title">Verify Email</h1>

		{#if verifying}
			<p class="verify-card__muted">Verifying your email...</p>
		{:else if success}
			<div class="verify-card__success">{message}</div>
			<a href="/login" class="verify-card__cta">Continue to sign in</a>
		{:else}
			<div class="verify-card__error">{error}</div>
			<div class="verify-card__actions">
				<a href="/login" class="verify-card__back">Back to sign in</a>
				<a href="/resend-verification" class="verify-card__back">Resend verification email</a>
			</div>
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
		text-align: center;
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
		margin-bottom: 1rem;
	}

	.verify-card__muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
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

	.verify-card__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		line-height: 1.5;
		margin-bottom: 1rem;
	}

	.verify-card__cta {
		display: inline-block;
		width: 100%;
		padding: 0.85rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		border-radius: var(--radius-lg);
		border: none;
		text-decoration: none;
		transition:
			opacity 200ms var(--ease-out),
			transform 200ms var(--ease-out);
	}

	.verify-card__cta:hover {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	.verify-card__actions {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.verify-card__back {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		background: transparent;
		border: none;
		cursor: pointer;
	}

	.verify-card__back:hover {
		color: var(--color-teal);
	}
</style>
