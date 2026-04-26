/**
 * Drawer type definitions.
 *
 * Co-located with `Drawer.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */
import type { Snippet } from 'svelte';

export type DrawerPosition = 'start' | 'end' | 'top' | 'bottom';
export type DrawerSize = 'sm' | 'md' | 'lg' | 'xl' | 'full';

export interface DrawerProps {
	open: boolean;
	onclose?: () => void;
	title: string;
	description?: string;
	size?: DrawerSize;
	position?: DrawerPosition;
	closeOnBackdrop?: boolean;
	closeOnEscape?: boolean;
	children: Snippet;
	footer?: Snippet;
}
