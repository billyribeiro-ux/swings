/**
 * Breadcrumbs type definitions.
 *
 * Co-located with `Breadcrumbs.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */

export interface BreadcrumbItem {
	label: string;
	href?: string;
}

export interface BreadcrumbsProps {
	items: BreadcrumbItem[];
	'aria-label'?: string;
}
