/**
 * PE7 shared UI primitive library.
 *
 * Canonical re-exports for every component. Types are exposed alongside the
 * component so consumers can import both from a single path, e.g.
 *
 *     import { Button, type ButtonVariant } from '$lib/components/shared';
 *
 * Type re-exports point at sibling `*.types.ts` files rather than `*.svelte`.
 * This keeps bare `tsc --noEmit` happy — the ambient `*.svelte` shim only
 * declares a default export, so named-type re-exports through the `.svelte`
 * file fail with TS2614 outside the Svelte language plugin.
 */

export { default as Button } from './Button.svelte';
export type { ButtonProps, ButtonSize, ButtonVariant } from './Button.types';

export { default as Dialog } from './Dialog.svelte';
export type { DialogProps, DialogSize } from './Dialog.types';

export { default as Drawer } from './Drawer.svelte';
export type { DrawerPosition, DrawerProps, DrawerSize } from './Drawer.types';

export { default as Toast } from './Toast.svelte';
export type { ToastKind, ToastProps } from './Toast.types';

export { default as ToastRegion } from './ToastRegion.svelte';
export type { ToastRegionPosition, ToastRegionProps } from './ToastRegion.types';

export { default as Stepper } from './Stepper.svelte';
export type { StepperProps, StepperStep } from './Stepper.types';

export { default as Spinner } from './Spinner.svelte';

export { default as FormField } from './FormField.svelte';
export type { FormFieldChildContext, FormFieldProps } from './FormField.types';

export { default as Breadcrumbs } from './Breadcrumbs.svelte';
export type { BreadcrumbItem, BreadcrumbsProps } from './Breadcrumbs.types';

export { default as ActionMenu } from './ActionMenu.svelte';
export { default as ActionMenuItem } from './ActionMenuItem.svelte';
export { default as ActionMenuDivider } from './ActionMenuDivider.svelte';

export { default as EmptyState } from './EmptyState.svelte';
export type { EmptyStateProps } from './EmptyState.types';

export { default as VisuallyHidden } from './VisuallyHidden.svelte';

export {
	ToastStore,
	toasts,
	type ToastInput,
	type ToastItem,
	type ToastKind as StoreToastKind
} from '$lib/stores/toasts.svelte';
