<!--
  FORM-10: ISO-8601 date picker. Browser's native `<input type="date">` is
  preferred — poor polyfills are worse than the native picker in every
  modern browser; validate.ts is the authoritative gate for min/max/range.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const d = $derived(field as Extract<FieldSchema, { type: 'date' }>);
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
			type="date"
			class="fm-input"
			value={current}
			{disabled}
			min={d.min_date}
			max={d.max_date}
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			oninput={handleInput}
		/>
	{/snippet}
</FieldFrame>
