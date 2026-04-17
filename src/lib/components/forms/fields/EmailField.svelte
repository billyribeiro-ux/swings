<!--
  FORM-10: RFC-5321 email input. `type="email"` drives the mobile keyboard;
  real validation is handled by validate.ts so the UX mirrors the server.
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
			type="email"
			class="fm-input"
			value={current}
			{disabled}
			placeholder={field.placeholder ?? 'you@example.com'}
			inputmode="email"
			autocomplete="email"
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			oninput={handleInput}
		/>
	{/snippet}
</FieldFrame>
