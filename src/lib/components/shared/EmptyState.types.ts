/**
 * EmptyState type definitions.
 *
 * Co-located with `EmptyState.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */
import type { Snippet } from 'svelte';

export interface EmptyStateProps {
	title: string;
	description?: string;
	icon?: Snippet;
	action?: Snippet;
	/** Element used as the title tag. Defaults to h2. */
	titleTag?: 'h1' | 'h2' | 'h3' | 'h4';
}
