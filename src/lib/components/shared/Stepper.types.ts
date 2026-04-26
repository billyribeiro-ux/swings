/**
 * Stepper type definitions.
 *
 * Co-located with `Stepper.svelte`. Lives in a `.ts` sibling so bare
 * `tsc --noEmit` (which doesn't speak Svelte) can resolve named-type
 * re-exports from `index.ts`.
 */

export interface StepperStep {
	id: string;
	label: string;
	description?: string;
}

export interface StepperProps {
	steps: StepperStep[];
	/** Id of the current (active) step. */
	current: string;
	/** Ids that have been completed (supports out-of-order flows). */
	completed?: string[];
	orientation?: 'horizontal' | 'vertical';
	/** When true, renders each step as a focusable button and fires `onselect`. */
	interactive?: boolean;
	onselect?: (id: string) => void;
	'aria-label'?: string;
}
