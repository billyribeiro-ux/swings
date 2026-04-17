<!--
  FORM-10: URL input. `type="url"` opens the URL keyboard on mobile;
  parseability is enforced by validate.ts via `new URL(...)`.
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
			type="url"
			class="fm-input"
			value={current}
			{disabled}
			placeholder={field.placeholder ?? 'https://example.com'}
			inputmode="url"
			autocomplete="url"
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			oninput={handleInput}
		/>
	{/snippet}
</FieldFrame>
