<!--
  FORM-10: Range slider. Exposes the live value via a paired `<output>` so
  users always know the current number (sliders without readouts fail
  WCAG 2.2's SC 1.3.5 Identify Input Purpose for numeric values).
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const sl = $derived(field as Extract<FieldSchema, { type: 'slider' }>);
	const midpoint = $derived((sl.min + sl.max) / 2);
	const current = $derived(
		typeof value === 'number' && Number.isFinite(value) ? value : midpoint
	);
	const controlId = $derived(`form-field-${field.key}`);

	function handleInput(e: Event) {
		const target = e.currentTarget as HTMLInputElement;
		const parsed = Number(target.value);
		if (Number.isFinite(parsed)) {
			onChange(field.key, parsed);
		}
	}

	$effect(() => {
		// Materialise the midpoint on first render so submit carries a value.
		if (value === undefined || value === null) {
			onChange(field.key, midpoint);
		}
	});
</script>

<FieldFrame
	{controlId}
	label={field.label ?? field.key}
	helpText={field.helpText}
	{error}
	required={field.required ?? false}
>
	{#snippet children({ describedBy, invalid, required: _required })}
		<div class="fm-slider">
			<input
				id={controlId}
				name={field.key}
				type="range"
				class="fm-slider__input"
				value={current}
				min={sl.min}
				max={sl.max}
				step={sl.step ?? 1}
				{disabled}
				aria-describedby={describedBy}
				aria-invalid={invalid}
				aria-valuemin={sl.min}
				aria-valuemax={sl.max}
				aria-valuenow={current}
				oninput={handleInput}
			/>
			<output class="fm-slider__output" for={controlId}>{current}</output>
		</div>
	{/snippet}
</FieldFrame>
