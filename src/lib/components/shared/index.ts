/**
 * PE7 shared UI primitive library.
 *
 * Canonical re-exports for every component. Types are exposed alongside the
 * component so consumers can import both from a single path, e.g.
 *
 *     import { Button, type ButtonVariant } from '$lib/components/shared';
 */

export { default as Button } from './Button.svelte';
export type { ButtonProps, ButtonSize, ButtonVariant } from './Button.svelte';

export { default as Dialog } from './Dialog.svelte';
export type { DialogProps, DialogSize } from './Dialog.svelte';

export { default as Drawer } from './Drawer.svelte';
export type { DrawerPosition, DrawerProps, DrawerSize } from './Drawer.svelte';

export { default as Toast } from './Toast.svelte';
export type { ToastKind, ToastProps } from './Toast.svelte';

export { default as ToastRegion } from './ToastRegion.svelte';
export type { ToastRegionPosition, ToastRegionProps } from './ToastRegion.svelte';

export { default as Stepper } from './Stepper.svelte';
export type { StepperProps, StepperStep } from './Stepper.svelte';

export { default as Spinner } from './Spinner.svelte';

export { default as FormField } from './FormField.svelte';
export type { FormFieldChildContext, FormFieldProps } from './FormField.svelte';

export { default as Breadcrumbs } from './Breadcrumbs.svelte';
export type { BreadcrumbItem, BreadcrumbsProps } from './Breadcrumbs.svelte';

export { default as EmptyState } from './EmptyState.svelte';
export type { EmptyStateProps } from './EmptyState.svelte';

export { default as VisuallyHidden } from './VisuallyHidden.svelte';

export {
	ToastStore,
	toasts,
	type ToastInput,
	type ToastItem,
	type ToastKind as StoreToastKind
} from '$lib/stores/toasts.svelte';
