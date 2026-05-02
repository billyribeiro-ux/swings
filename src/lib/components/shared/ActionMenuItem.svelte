<script lang="ts">
	/**
	 * Generic action-menu item primitive. When `href` is supplied the consumer
	 * is responsible for resolving typed routes via SvelteKit's `resolve()`
	 * helper — this primitive cannot statically know which typed route is
	 * being passed, so it accepts a pre-resolved string.
	 */
	import type { Component, Snippet } from 'svelte';
	import type { SVGAttributes } from 'svelte/elements';

	type Variant = 'default' | 'danger';

	type IconWeight = 'bold' | 'duotone' | 'fill' | 'light' | 'thin' | 'regular';

	/**
	 * Structural mirror of `IconComponentProps` from `phosphor-svelte/lib/shared`
	 * (which is not a public sub-path export). Keeps the icon prop fully
	 * compatible with every Phosphor `*Icon` component without forcing the
	 * consumer to type-assert.
	 */
	interface IconBaseProps {
		color?: string;
		size?: number | string;
		weight?: IconWeight;
		mirrored?: boolean;
	}
	interface IconComponentProps
		extends Omit<SVGAttributes<SVGSVGElement>, keyof IconBaseProps>,
			IconBaseProps {}

	interface Props {
		icon?: Component<IconComponentProps, Record<string, never>, ''> | undefined;
		variant?: Variant | undefined;
		disabled?: boolean | undefined;
		onclick?: ((event: MouseEvent) => void) | undefined;
		/**
		 * When set, the item renders as a real `<a>` element so middle-click /
		 * cmd-click open the target in a new tab — the standard browser
		 * affordance for navigation. Mutually exclusive with `onclick` in
		 * intent, but both can coexist if the consumer needs e.g. an
		 * analytics ping on follow.
		 */
		href?: string | undefined;
		children: Snippet;
	}

	let {
		icon,
		variant = 'default',
		disabled = false,
		onclick,
		href,
		children
	}: Props = $props();

	function handleClick(event: MouseEvent) {
		if (disabled) {
			event.preventDefault();
			event.stopPropagation();
			return;
		}
		onclick?.(event);
	}
</script>

{#if href}
	<svelte:element
		this={'a'}
		role="menuitem"
		tabindex="-1"
		class="action-menu__item action-menu__item--{variant}"
		aria-disabled={disabled ? 'true' : undefined}
		{href}
		onclick={handleClick}
	>
		{#if icon}
			{@const Icon = icon}
			<span class="action-menu__icon" aria-hidden="true">
				<Icon size={16} weight="bold" />
			</span>
		{/if}
		<span class="action-menu__label">{@render children()}</span>
	</svelte:element>
{:else}
	<button
		type="button"
		role="menuitem"
		tabindex="-1"
		class="action-menu__item action-menu__item--{variant}"
		aria-disabled={disabled ? 'true' : undefined}
		onclick={handleClick}
	>
		{#if icon}
			{@const Icon = icon}
			<span class="action-menu__icon" aria-hidden="true">
				<Icon size={16} weight="bold" />
			</span>
		{/if}
		<span class="action-menu__label">{@render children()}</span>
	</button>
{/if}

<style>
	.action-menu__item {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
		width: 100%;
		padding: 0.6rem 0.9rem;
		background: transparent;
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-grey-200);
		font-family: var(--font-ui);
		font-size: 0.8125rem;
		font-weight: var(--w-medium);
		line-height: 1.3;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
		box-sizing: border-box;
		transition:
			background-color 120ms var(--ease-out),
			color 120ms var(--ease-out);
	}

	.action-menu__item:hover:not([aria-disabled='true']),
	.action-menu__item:focus-visible:not([aria-disabled='true']) {
		background: rgba(15, 164, 175, 0.08);
		color: var(--color-white);
		outline: none;
	}

	.action-menu__item[aria-disabled='true'] {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.action-menu__item--danger {
		color: oklch(72% 0.16 25);
	}

	.action-menu__item--danger:hover:not([aria-disabled='true']),
	.action-menu__item--danger:focus-visible:not([aria-disabled='true']) {
		background: rgba(239, 68, 68, 0.1);
		color: oklch(78% 0.18 25);
	}

	.action-menu__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.action-menu__label {
		flex: 1 1 auto;
		min-width: 0;
	}
</style>
