<script lang="ts">
	import { page } from '$app/state';
	import WarningCircleIcon from 'phosphor-svelte/lib/WarningCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import LockKeyIcon from 'phosphor-svelte/lib/LockKeyIcon';
	import ArrowClockwiseIcon from 'phosphor-svelte/lib/ArrowClockwiseIcon';
	import HouseIcon from 'phosphor-svelte/lib/HouseIcon';
	import SignInIcon from 'phosphor-svelte/lib/SignInIcon';

	const status = $derived(page.status);
	// Strip multi-line stack traces / leaked file paths so the visible message
	// stays a single sanitised sentence. Server-side errors sometimes carry
	// `Error: foo\n    at fn (file:line:col)` shaped strings — only the first
	// non-empty line is user-friendly.
	function sanitizeMessage(raw: string | undefined): string {
		if (!raw) return 'Something went wrong.';
		const firstLine = raw
			.split(/\r?\n/)
			.find((line) => line.trim().length > 0)
			?.trim();
		if (!firstLine) return 'Something went wrong.';
		// Trim a trailing stack-frame fragment like " (at fn (...))" if present.
		return firstLine.replace(/\s+at\s+.*$/i, '').slice(0, 240);
	}
	const message = $derived(sanitizeMessage(page.error?.message));

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

	const Icon = $derived(
		status === 404
			? MagnifyingGlassIcon
			: status === 403
				? XCircleIcon
				: status === 401
					? LockKeyIcon
					: WarningCircleIcon
	);
</script>

<svelte:head>
	<title>{title} · Dashboard · Swings</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<section class="dash-error" aria-labelledby="dash-error-heading" aria-live="polite">
	<div class="dash-error__card">
		<div class="dash-error__icon" aria-hidden="true">
			<Icon size={36} weight="duotone" />
		</div>
		<p class="dash-error__status">{status}</p>
		<h1 id="dash-error-heading" class="dash-error__title">{title}</h1>
		<p class="dash-error__body">{body}</p>

		{#if correlationId}
			<p class="dash-error__meta" aria-label="Error reference">
				Reference: <code>{correlationId}</code>
			</p>
		{/if}

		<div class="dash-error__actions">
			{#if status === 401}
				<a class="dash-error__btn dash-error__btn--primary" href="/login">
					<SignInIcon size={16} weight="bold" />
					<span>Sign in</span>
				</a>
			{:else}
				<button
					type="button"
					class="dash-error__btn dash-error__btn--primary"
					onclick={() => location.reload()}
				>
					<ArrowClockwiseIcon size={16} weight="bold" />
					<span>Try again</span>
				</button>
			{/if}
			<a class="dash-error__btn dash-error__btn--secondary" href="/dashboard">
				<HouseIcon size={16} weight="bold" />
				<span>Go to dashboard</span>
			</a>
		</div>
	</div>
</section>

<style>
	.dash-error {
		display: grid;
		place-items: center;
		min-height: calc(100dvh - 10rem);
		padding: 2rem 1rem;
		background-color: var(--color-navy-deep);
	}

	.dash-error__card {
		max-width: 32rem;
		width: 100%;
		padding: 2rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-align: center;
	}

	.dash-error__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 3.5rem;
		height: 3.5rem;
		margin: 0 auto 1rem;
		border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal-light);
	}

	.dash-error__status {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.75rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--color-teal);
		margin: 0 0 0.5rem;
	}

	.dash-error__title {
		font-size: clamp(1.5rem, 3.5vw, 2rem);
		font-weight: 700;
		font-family: var(--font-heading);
		margin: 0 0 0.75rem;
		color: var(--color-white);
		letter-spacing: -0.01em;
	}

	.dash-error__body {
		color: var(--color-grey-300);
		font-size: 0.9375rem;
		line-height: 1.6;
		margin: 0 0 1.5rem;
	}

	.dash-error__meta {
		font-size: 0.8125rem;
		color: var(--color-grey-400);
		margin: 0 0 1.5rem;
	}

	.dash-error__meta code {
		background-color: rgba(255, 255, 255, 0.06);
		padding: 0.15rem 0.4rem;
		border-radius: var(--radius-sm);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.8125rem;
		color: var(--color-white);
		user-select: all;
	}

	.dash-error__actions {
		display: flex;
		gap: 0.625rem;
		justify-content: center;
		flex-wrap: wrap;
	}

	.dash-error__btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.625rem 1rem;
		border-radius: var(--radius-lg);
		font-size: 0.8125rem;
		font-weight: 600;
		text-decoration: none;
		cursor: pointer;
		border: 1px solid transparent;
		transition:
			background-color 160ms var(--ease-out),
			border-color 160ms var(--ease-out),
			color 160ms var(--ease-out),
			transform 160ms var(--ease-out);
	}

	.dash-error__btn:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	.dash-error__btn--primary {
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
	}

	.dash-error__btn--primary:hover {
		opacity: 0.92;
		transform: translateY(-1px);
	}

	.dash-error__btn--secondary {
		background-color: rgba(255, 255, 255, 0.04);
		color: var(--color-white);
		border-color: rgba(255, 255, 255, 0.1);
	}

	.dash-error__btn--secondary:hover {
		background-color: rgba(255, 255, 255, 0.08);
		border-color: rgba(255, 255, 255, 0.16);
	}
</style>
