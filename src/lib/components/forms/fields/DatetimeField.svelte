<!--
  FORM-10: RFC-3339 datetime-local input. The browser emits the local ISO
  string; validate.ts accepts anything `Date.parse` can read so local vs. Z
  suffix is normalised server-side.
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
			type="datetime-local"
			class="fm-input"
			value={current}
			{disabled}
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			oninput={handleInput}
		/>
	{/snippet}
</FieldFrame>
