<!--
  FORM-10: Multi-line textarea. Length constraints are declarative via the
  HTML attributes; validate.ts re-checks authoritatively.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const textarea = $derived(field as Extract<FieldSchema, { type: 'textarea' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);

	function handleInput(e: Event) {
		const target = e.currentTarget as HTMLTextAreaElement;
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
		<textarea
			id={controlId}
			name={field.key}
			class="fm-textarea"
			{disabled}
			placeholder={field.placeholder ?? ''}
			minlength={textarea.min_length}
			maxlength={textarea.max_length}
			rows={5}
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			oninput={handleInput}>{current}</textarea
		>
	{/snippet}
</FieldFrame>
