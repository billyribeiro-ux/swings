<!--
  Button — canonical PE7 primitive. Renders as `<button>` by default; when
  `href` is provided it renders as `<a>` and accepts anchor semantics.

  A11y:
  - `disabled` disables both button and anchor variants (anchor gets
    `aria-disabled="true"` + `role="link"` semantics preserved, `tabindex="-1"`).
  - `loading` sets `aria-busy="true"` and disables interaction.
  - Focus ring delegated to the global `:focus-visible` style (teal 2px outline,
    configured in `base.css`).
  - Phosphor icons (optional snippet slots) are marked `aria-hidden` via
    container so the visible label remains the accessible name.
-->
<script lang="ts" module>
	export type { ButtonProps, ButtonSize, ButtonVariant } from './Button.types';
</script>

<script lang="ts">
	import Spinner from './Spinner.svelte';
	import type { ButtonProps as Props } from './Button.types';

	const {
		variant = 'primary',
		size = 'md',
		href,
		type = 'button',
		target,
		rel,
		disabled = false,
		loading = false,
		fullWidth = false,
		onclick,
		iconLeading,
		iconTrailing,
		children,
		'aria-label': ariaLabel
	}: Props = $props();

	const isInactive = $derived(disabled || loading);
	// Auto-assign `rel="noopener noreferrer"` for _blank anchors if unset.
	const anchorRel = $derived(
		target === '_blank' && rel === undefined ? 'noopener noreferrer' : rel
	);

	function handleClick(e: MouseEvent) {
		if (isInactive) {
			e.preventDefault();
			e.stopPropagation();
			return;
		}
		onclick?.(e);
	}
</script>

{#snippet inner()}
	{#if loading}
		<span class="icon leading" aria-hidden="true">
			<Spinner size={size === 'lg' ? 'md' : 'sm'} label="" inline />
		</span>
	{:else if iconLeading}
		<span class="icon leading" aria-hidden="true">{@render iconLeading()}</span>
	{/if}
	<span class="label">{@render children()}</span>
	{#if iconTrailing && !loading}
		<span class="icon trailing" aria-hidden="true">{@render iconTrailing()}</span>
	{/if}
{/snippet}

{#if href}
	<a
		class="btn"
		data-variant={variant}
		data-size={size}
		class:full-width={fullWidth}
		class:loading
		href={isInactive ? undefined : href}
		{target}
		rel={anchorRel}
		aria-disabled={isInactive || undefined}
		aria-busy={loading || undefined}
		aria-label={ariaLabel}
		tabindex={isInactive ? -1 : undefined}
		onclick={handleClick}
	>
		{@render inner()}
	</a>
{:else}
	<button
		class="btn"
		data-variant={variant}
		data-size={size}
		class:full-width={fullWidth}
		class:loading
		{type}
		disabled={isInactive}
		aria-busy={loading || undefined}
		aria-label={ariaLabel}
		onclick={handleClick}
	>
		{@render inner()}
	</button>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		border: 1px solid transparent;
		border-radius: var(--radius-lg);
		font-family: var(--font-ui);
		font-weight: var(--w-semibold);
		line-height: var(--lh-snug);
		letter-spacing: var(--ls-normal);
		text-decoration: none;
		cursor: pointer;
		user-select: none;
		white-space: nowrap;
		transition:
			background-color var(--duration-150) var(--ease-out),
			color var(--duration-150) var(--ease-out),
			border-color var(--duration-150) var(--ease-out),
			box-shadow var(--duration-150) var(--ease-out),
			transform var(--duration-100) var(--ease-out);
	}

	.btn.full-width {
		display: flex;
		inline-size: 100%;
	}

	.btn[data-size='sm'] {
		font-size: var(--fs-xs);
		padding-block: var(--space-1-5);
		padding-inline: var(--space-3);
		min-block-size: 2rem;
	}
	.btn[data-size='md'] {
		font-size: var(--fs-sm);
		padding-block: var(--space-2);
		padding-inline: var(--space-4);
		min-block-size: 2.5rem;
	}
	.btn[data-size='lg'] {
		font-size: var(--fs-md);
		padding-block: var(--space-3);
		padding-inline: var(--space-5);
		min-block-size: 3rem;
	}

	/* Variants */
	.btn[data-variant='primary'] {
		background-color: var(--brand-teal-500);
		color: var(--neutral-0);
		border-color: var(--brand-teal-500);
	}
	.btn[data-variant='primary']:hover:not([disabled]):not([aria-disabled='true']) {
		background-color: var(--brand-teal-600);
		border-color: var(--brand-teal-600);
	}

	.btn[data-variant='secondary'] {
		background-color: var(--brand-navy-700);
		color: var(--neutral-0);
		border-color: var(--brand-navy-700);
	}
	.btn[data-variant='secondary']:hover:not([disabled]):not([aria-disabled='true']) {
		background-color: var(--brand-navy-600);
		border-color: var(--brand-navy-600);
	}

	.btn[data-variant='tertiary'] {
		background-color: transparent;
		color: var(--surface-fg-default);
		border-color: var(--surface-border-default);
	}
	.btn[data-variant='tertiary']:hover:not([disabled]):not([aria-disabled='true']) {
		background-color: var(--surface-bg-muted);
		border-color: var(--surface-border-default);
	}

	.btn[data-variant='danger'] {
		background-color: var(--status-danger-500);
		color: var(--neutral-0);
		border-color: var(--status-danger-500);
	}
	.btn[data-variant='danger']:hover:not([disabled]):not([aria-disabled='true']) {
		background-color: var(--status-danger-700);
		border-color: var(--status-danger-700);
	}

	.btn[data-variant='ghost'] {
		background-color: transparent;
		color: var(--surface-fg-default);
		border-color: transparent;
	}
	.btn[data-variant='ghost']:hover:not([disabled]):not([aria-disabled='true']) {
		background-color: var(--surface-bg-muted);
	}

	.btn[data-variant='link'] {
		background-color: transparent;
		color: var(--brand-teal-500);
		border-color: transparent;
		padding-inline: var(--space-0-5);
		min-block-size: auto;
		text-decoration: underline;
		text-decoration-thickness: 1px;
		text-underline-offset: 0.2em;
	}
	.btn[data-variant='link']:hover:not([disabled]):not([aria-disabled='true']) {
		color: var(--brand-teal-600);
	}

	.btn:active:not([disabled]):not([aria-disabled='true']) {
		transform: translateY(1px);
	}

	.btn:disabled,
	.btn[aria-disabled='true'] {
		opacity: 0.55;
		cursor: not-allowed;
		pointer-events: none;
	}

	.btn.loading {
		cursor: progress;
	}

	.icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		line-height: 0;
	}

	@media (prefers-reduced-motion: reduce) {
		.btn {
			transition: none;
		}
		.btn:active:not([disabled]):not([aria-disabled='true']) {
			transform: none;
		}
	}
</style>
