<!--
  FormField — wrapper around a form control that wires label + description +
  error to the control via ARIA attributes.

  A11y:
  - `<label for={forId}>` always rendered (no placeholder-only labels).
  - Description has a deterministic id so consumers can use `aria-describedby`.
  - Error region is `role="alert"` + `aria-live="polite"` so changes are
    announced without stealing focus.
  - Required is indicated visually AND via an sr-only "required" string; the
    `*` glyph alone is insufficient for screen readers.

  The `children` snippet receives `{ describedBy, invalid, required }` which
  callers spread / pass through to the underlying control.
-->
<script lang="ts" module>
	export type { FormFieldChildContext, FormFieldProps } from './FormField.types';
</script>

<script lang="ts">
	import type { FormFieldProps as Props } from './FormField.types';

	const {
		for: forId,
		label,
		description,
		error,
		required = false,
		children
	}: Props = $props();

	const descriptionId = $derived(description ? `${forId}-desc` : undefined);
	const errorId = $derived(error ? `${forId}-err` : undefined);
	const describedBy = $derived(
		[descriptionId, errorId].filter((s): s is string => typeof s === 'string').join(' ') ||
			undefined
	);
	const invalid = $derived(!!error);
</script>

<div class="form-field" class:invalid>
	<label for={forId} class="label">
		{label}
		{#if required}
			<span class="required-mark" aria-hidden="true">*</span>
			<span class="visually-hidden"> (required)</span>
		{/if}
	</label>
	{#if description}
		<p id={descriptionId} class="description">{description}</p>
	{/if}
	<div class="control">
		{@render children({ describedBy, invalid, required })}
	</div>
	{#if error}
		<p id={errorId} class="error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>

<style>
	.form-field { display: flex; flex-direction: column; gap: var(--space-1-5); }
	.label { font-size: var(--fs-sm); font-weight: var(--w-medium); color: var(--surface-fg-default); line-height: var(--lh-snug); }
	.required-mark { color: var(--status-danger-500); margin-inline-start: var(--space-0-5); }
	.description { margin: 0; font-size: var(--fs-xs); color: var(--surface-fg-muted); line-height: var(--lh-normal); }
	.control { display: flex; flex-direction: column; }
	.form-field.invalid :global(input),
	.form-field.invalid :global(textarea),
	.form-field.invalid :global(select) {
		border-color: var(--status-danger-500);
	}
	.error { margin: 0; font-size: var(--fs-xs); color: var(--status-danger-700); line-height: var(--lh-normal); }
	.visually-hidden { position: absolute; inline-size: 1px; block-size: 1px; padding: 0; margin: -1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; border: 0; }
</style>
