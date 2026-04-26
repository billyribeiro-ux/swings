/**
 * ToastRegion type definitions.
 *
 * Co-located with `ToastRegion.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */
import type { ToastStore } from '$lib/stores/toasts.svelte';

export type ToastRegionPosition =
	| 'top-start'
	| 'top-end'
	| 'top-center'
	| 'bottom-start'
	| 'bottom-end'
	| 'bottom-center';

export interface ToastRegionProps {
	position?: ToastRegionPosition;
	/** Optional custom store — defaults to the shared `toasts` instance. */
	store?: ToastStore;
	label?: string;
}
