<!--
  FORM-10: Single-line text input.
  Wires length + pattern via HTML constraints; the validate.ts pipeline is
  still the source of truth for error codes shown to the user.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();

	// Narrow variant so pattern / length rules are statically accessible.
	const text = $derived(field as Extract<FieldSchema, { type: 'text' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);

	function handleInput(e: Event) {
		const target = e.currentTarget as HTMLInputElement;
		onChange(field.key, target.value);
	}
</script>

<FieldFrame
	{controlId}
	label={field.label ?? field.key}
	helpText={field.helpText}
	{error}
	required={field.required ?? false}
>
	{#snippet children({ describedBy, invalid, required })}
		<input
			id={controlId}
			name={field.key}
			type="text"
			class="fm-input"
			value={current}
			{disabled}
			placeholder={field.placeholder ?? ''}
			minlength={text.min_length}
			maxlength={text.max_length}
			pattern={text.pattern}
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			autocomplete="off"
			oninput={handleInput}
		/>
	{/snippet}
</FieldFrame>
