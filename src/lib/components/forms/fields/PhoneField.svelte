<!--
  FORM-10: Phone input. E.164 format is enforced by validate.ts; we hint the
  mobile dial-pad keyboard via `inputmode="tel"`.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
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
			type="tel"
			class="fm-input"
			value={current}
			{disabled}
			placeholder={field.placeholder ?? '+14155551234'}
			inputmode="tel"
			autocomplete="tel"
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			oninput={handleInput}
		/>
	{/snippet}
</FieldFrame>
