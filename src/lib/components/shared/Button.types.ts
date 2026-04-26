/**
 * Button type definitions.
 *
 * Co-located with `Button.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */
import type { Snippet } from 'svelte';
import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

export type ButtonVariant = 'primary' | 'secondary' | 'tertiary' | 'danger' | 'ghost' | 'link';
export type ButtonSize = 'sm' | 'md' | 'lg';

export interface ButtonProps {
	variant?: ButtonVariant;
	size?: ButtonSize;
	/** Render as `<a>` when set. Otherwise `<button>`. */
	href?: string;
	type?: HTMLButtonAttributes['type'];
	target?: HTMLAnchorAttributes['target'];
	rel?: HTMLAnchorAttributes['rel'];
	disabled?: boolean;
	loading?: boolean;
	fullWidth?: boolean;
	onclick?: (e: MouseEvent) => void;
	iconLeading?: Snippet;
	iconTrailing?: Snippet;
	children: Snippet;
	/** Accessible label when the button has no visible text (icon-only). */
	'aria-label'?: string;
}
