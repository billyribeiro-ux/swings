<script lang="ts">
	import { page } from '$app/state';

	const status = $derived(page.status);
	const message = $derived(page.error?.message ?? 'Something went wrong.');

	const title = $derived(
		status === 404
			? 'Page not found'
			: status === 403
				? 'Access denied'
				: status === 401
					? 'Sign in required'
					: status >= 500
						? 'Something went wrong'
						: 'Request failed'
	);

	const body = $derived(
		status === 404
			? "We couldn't find the page you're looking for. It may have moved or no longer exist."
			: status === 403
				? "You don't have permission to view this page."
				: status === 401
					? 'Please sign in to continue.'
					: status >= 500
						? 'Our servers hit a snag. The team has been notified — please try again in a moment.'
						: message
	);

	// `(page.error as { id?: string })` because SvelteKit's HandleServerError
	// allows extending the error object with a correlation id — we emit one in
	// `hooks.server.ts`. Typed this way so we don't widen App.Error globally.
	const correlationId = $derived((page.error as { id?: string } | null)?.id ?? null);
</script>

<svelte:head>
	<title>{title} · Swings</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<main class="error-page" id="main-content" aria-labelledby="error-heading" aria-live="polite">
	<div class="error-page__card">
		<p class="error-page__status">{status}</p>
		<h1 id="error-heading" class="error-page__title">{title}</h1>
		<p class="error-page__body">{body}</p>

		{#if correlationId}
			<p class="error-page__meta" aria-label="Error reference">
				Reference: <code>{correlationId}</code>
			</p>
		{/if}

		<div class="error-page__actions">
			<a class="error-page__btn error-page__btn--primary" href="/">Go home</a>
			{#if status === 401}
				<a class="error-page__btn error-page__btn--secondary" href="/login">Sign in</a>
			{:else}
				<button
					type="button"
					class="error-page__btn error-page__btn--secondary"
					onclick={() => location.reload()}
				>
					Try again
				</button>
			{/if}
		</div>
	</div>
</main>

<style>
	.error-page {
		min-height: calc(100dvh - 4rem);
		display: grid;
		place-items: center;
		padding: 3rem 1rem;
		background: var(--color-bg, #0b0f14);
	}

	.error-page__card {
		max-width: 32rem;
		width: 100%;
		padding: 2.5rem;
		background: var(--color-surface, rgba(255, 255, 255, 0.04));
		border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
		border-radius: 1rem;
		text-align: center;
	}

	.error-page__status {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.875rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--color-teal, #0fa4af);
		margin: 0 0 0.5rem;
	}

	.error-page__title {
		font-size: clamp(1.75rem, 4vw, 2.5rem);
		font-weight: 700;
		margin: 0 0 0.75rem;
		color: var(--color-white, #fff);
	}

	.error-page__body {
		color: var(--color-grey-300, rgba(255, 255, 255, 0.75));
		line-height: 1.6;
		margin: 0 0 1.5rem;
	}

	.error-page__meta {
		font-size: 0.875rem;
		color: var(--color-grey-400, rgba(255, 255, 255, 0.55));
		margin: 0 0 1.75rem;
	}
	.error-page__meta code {
		background: rgba(255, 255, 255, 0.06);
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
	}

	.error-page__actions {
		display: flex;
		gap: 0.75rem;
		justify-content: center;
		flex-wrap: wrap;
	}

	.error-page__btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.625rem 1.125rem;
		border-radius: 0.5rem;
		font-weight: 600;
		font-size: 0.9375rem;
		cursor: pointer;
		border: 1px solid transparent;
		text-decoration: none;
		transition:
			transform 150ms ease,
			background 150ms ease;
	}
	.error-page__btn:focus-visible {
		outline: 2px solid var(--color-teal, #0fa4af);
		outline-offset: 2px;
	}
	.error-page__btn--primary {
		background: var(--color-teal, #0fa4af);
		color: #fff;
	}
	.error-page__btn--primary:hover {
		background: var(--color-teal-600, #0d8c95);
	}
	.error-page__btn--secondary {
		background: transparent;
		color: var(--color-white, #fff);
		border-color: var(--color-border, rgba(255, 255, 255, 0.15));
	}
	.error-page__btn--secondary:hover {
		background: rgba(255, 255, 255, 0.05);
	}
</style>
