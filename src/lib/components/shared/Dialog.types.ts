/**
 * Dialog type definitions.
 *
 * Co-located with `Dialog.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */
import type { Snippet } from 'svelte';

export type DialogSize = 'sm' | 'md' | 'lg' | 'xl' | 'full';

export interface DialogProps {
	open: boolean;
	onclose?: () => void;
	title: string;
	description?: string;
	size?: DialogSize;
	/** Close when the backdrop is clicked. Default `true`. */
	closeOnBackdrop?: boolean;
	/** Close when Escape is pressed. Default `true`. */
	closeOnEscape?: boolean;
	children: Snippet;
	/** Optional footer snippet (rendered in a flex footer row). */
	footer?: Snippet;
}
