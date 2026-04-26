/**
 * FormField type definitions.
 *
 * Co-located with `FormField.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */
import type { Snippet } from 'svelte';

export interface FormFieldChildContext {
	describedBy: string | undefined;
	invalid: boolean;
	required: boolean;
}

export interface FormFieldProps {
	for: string;
	label: string;
	description?: string | undefined;
	error?: string | undefined;
	required?: boolean | undefined;
	children: Snippet<[FormFieldChildContext]>;
}
