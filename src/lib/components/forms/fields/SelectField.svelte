<!--
  FORM-10: Single-choice dropdown. Renders a `<select>` so the browser's
  native picker (mobile OS wheel on iOS; dropdown on desktop) is always used.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const sel = $derived(field as Extract<FieldSchema, { type: 'select' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);

	function handleChange(e: Event) {
		const target = e.currentTarget as HTMLSelectElement;
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
		<select
			id={controlId}
			name={field.key}
			class="fm-select"
			value={current}
			{disabled}
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			onchange={handleChange}
		>
			<option value="" disabled={required}>{field.placeholder ?? 'Select an option'}</option>
			{#each sel.options as opt (opt.value)}
				<option value={opt.value} disabled={opt.disabled ?? false}>{opt.label}</option>
			{/each}
		</select>
	{/snippet}
</FieldFrame>
