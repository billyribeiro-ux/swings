/**
 * Toast type definitions.
 *
 * Co-located with `Toast.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */
import type { Snippet } from 'svelte';

export type ToastKind = 'info' | 'success' | 'warning' | 'danger';

export interface ToastProps {
	id?: string;
	kind?: ToastKind;
	title: string;
	description?: string;
	/** Auto-dismiss after this many ms. `0` = persistent. Default 5000. */
	duration?: number;
	onclose?: (id?: string) => void;
	icon?: Snippet;
}
