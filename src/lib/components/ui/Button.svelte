<script lang="ts">
	import { type Snippet } from 'svelte';

	interface Props {
		variant?: 'primary' | 'ghost' | 'outline';
		href?: string;
		onclick?: () => void;
		disabled?: boolean;
		children: Snippet;
	}

	let { variant = 'primary', href, onclick, disabled = false, children }: Props = $props();

	const classes = $derived(`btn btn--${variant}`);
</script>

{#if href}
	<a {href} class={classes} aria-disabled={disabled || undefined}>
		{@render children()}
	</a>
{:else}
	<button {onclick} {disabled} class={classes}>
		{@render children()}
	</button>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		border-radius: var(--radius-xl);
		font-family: var(--font-ui);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		transition: all 300ms cubic-bezier(0, 0, 0.2, 1);
		cursor: pointer;
		text-decoration: none;
		line-height: 1.5;
	}

	.btn:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-navy),
			0 0 0 4px rgba(15, 164, 175, 0.7);
	}

	.btn:active {
		transform: scale(0.97);
	}

	.btn:disabled,
	.btn[aria-disabled='true'] {
		pointer-events: none;
		opacity: 0.5;
	}

	.btn--primary {
		background-color: var(--color-teal);
		color: var(--color-white);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
	}

	.btn--primary:hover {
		background-color: var(--color-teal-light);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-xl),
			0 8px 20px rgba(15, 164, 175, 0.3);
	}

	.btn--ghost {
		background-color: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		border: 1px solid rgba(255, 255, 255, 0.15);
		backdrop-filter: blur(4px);
	}

	.btn--ghost:hover {
		background-color: rgba(255, 255, 255, 0.15);
		transform: translateY(-1px);
	}

	.btn--outline {
		background-color: transparent;
		color: var(--color-navy);
		border: 2px solid rgba(11, 29, 58, 0.8);
	}

	.btn--outline:hover {
		background-color: var(--color-navy);
		color: var(--color-white);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(11, 29, 58, 0.15);
	}
</style>
