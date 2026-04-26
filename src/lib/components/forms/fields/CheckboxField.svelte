<!--
  FORM-10: Single boolean checkbox (individual agreement, not a group — use
  Radio / MultiSelect for groups). Renders label to the right of the input
  per GOV.UK pattern.
-->
<script lang="ts">
	import type { FieldProps } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const checked = $derived(value === true);
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);

	function handleChange(e: Event) {
		const target = e.currentTarget as HTMLInputElement;
		onChange(field.key, target.checked);
	}
</script>

<div class="fm-field fm-field--check" class:fm-field--invalid={!!error}>
	<label class="fm-check" for={controlId}>
		<input
			id={controlId}
			name={field.key}
			type="checkbox"
			class="fm-check__input"
			{checked}
			{disabled}
			aria-describedby={describedBy}
			aria-invalid={!!error}
			aria-required={field.required ?? false}
			onchange={handleChange}
		/>
		<span class="fm-check__label">
			{field.label ?? field.key}
			{#if field.required}
				<span class="fm-field__required" aria-hidden="true">*</span>
				<span class="fm-field__sr">(required)</span>
			{/if}
		</span>
	</label>
	{#if field.helpText}
		<p id={helpId} class="fm-field__help">{field.helpText}</p>
	{/if}
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>
