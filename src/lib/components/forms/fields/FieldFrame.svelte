<!--
  FORM-10: Shared label + description + error scaffold.

  Every leaf field renders its control inside this frame so a11y wiring is
  consistent: label points at the control via `for`, error + description ids
  are surfaced to callers for `aria-describedby`, `role="alert"` drives
  `aria-live` announcements without stealing focus. Required visible and
  sr-only (the `*` glyph alone is not announced).

  The child snippet receives the deterministic ids so the control can wire
  its own `aria-describedby` / `aria-invalid` / `aria-required` without the
  parent pre-rendering them.
-->
<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export interface FieldFrameChildContext {
		readonly controlId: string;
		readonly describedBy: string | undefined;
		readonly invalid: boolean;
		readonly required: boolean;
	}

	export interface FieldFrameProps {
		readonly controlId: string;
		readonly label: string;
		readonly helpText?: string | undefined;
		readonly error?: string | undefined;
		readonly required?: boolean | undefined;
		readonly hideLabel?: boolean | undefined;
		readonly children: Snippet<[FieldFrameChildContext]>;
	}
</script>

<script lang="ts">
	const {
		controlId,
		label,
		helpText,
		error,
		required = false,
		hideLabel = false,
		children
	}: FieldFrameProps = $props();

	const helpId = $derived(helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);
	const invalid = $derived(!!error);
</script>

<div class="fm-field" class:fm-field--invalid={invalid}>
	<label for={controlId} class="fm-field__label" class:fm-field__label--hidden={hideLabel}>
		<span>{label}</span>
		{#if required}
			<span class="fm-field__required" aria-hidden="true">*</span>
			<span class="fm-field__sr">(required)</span>
		{/if}
	</label>
	{#if helpText}
		<p id={helpId} class="fm-field__help">{helpText}</p>
	{/if}
	<div class="fm-field__control">
		{@render children({ controlId, describedBy, invalid, required })}
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>
