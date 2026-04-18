<!--
  FORM-10: Numeric text input. Stores the raw string when empty so we never
  coerce `""` into `0`; the typed value is set once the user completes a valid
  number. validate.ts re-parses with `Number()` for authoritative validation.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const num = $derived(field as Extract<FieldSchema, { type: 'number' }>);
	const controlId = $derived(`form-field-${field.key}`);
	const current = $derived(
		typeof value === 'number' && Number.isFinite(value)
			? String(value)
			: typeof value === 'string'
				? value
				: ''
	);

	function handleInput(e: Event) {
		const target = e.currentTarget as HTMLInputElement;
		const raw = target.value;
		if (raw.length === 0) {
			onChange(field.key, '');
			return;
		}
		const parsed = Number(raw);
		if (Number.isFinite(parsed)) {
			onChange(field.key, parsed);
		} else {
			onChange(field.key, raw);
		}
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
			type="number"
			class="fm-input"
			value={current}
			{disabled}
			placeholder={field.placeholder ?? ''}
			min={num.min}
			max={num.max}
			step={num.step ?? 'any'}
			inputmode="decimal"
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			oninput={handleInput}
		/>
	{/snippet}
</FieldFrame>
