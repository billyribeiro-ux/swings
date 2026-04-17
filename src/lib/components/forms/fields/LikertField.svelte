<!--
  FORM-10: Likert scale. 1..=scale horizontal radio group with numeric +
  "strongly disagree / strongly agree" endpoint hints.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const l = $derived(field as Extract<FieldSchema, { type: 'likert' }>);
	const scale = $derived(l.scale ?? 5);
	const current = $derived(typeof value === 'number' && Number.isInteger(value) ? value : 0);
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);
	const values = $derived(Array.from({ length: scale }, (_, i) => i + 1));
</script>

<fieldset
	class="fm-field fm-field--group fm-field--likert"
	class:fm-field--invalid={!!error}
	aria-describedby={describedBy}
>
	<legend class="fm-field__label">
		{field.label ?? field.key}
		{#if field.required}
			<span class="fm-field__required" aria-hidden="true">*</span>
			<span class="fm-field__sr">(required)</span>
		{/if}
	</legend>
	{#if field.helpText}
		<p id={helpId} class="fm-field__help">{field.helpText}</p>
	{/if}
	<div class="fm-nps" role="radiogroup" aria-label={field.label ?? 'Likert scale'}>
		{#each values as n (n)}
			<label class="fm-nps__tile" class:fm-nps__tile--selected={n === current}>
				<input
					type="radio"
					class="fm-nps__input"
					name={field.key}
					value={n}
					checked={n === current}
					{disabled}
					onchange={() => onChange(field.key, n)}
				/>
				<span>{n}</span>
			</label>
		{/each}
	</div>
	<div class="fm-nps__legend">
		<span>Strongly disagree</span>
		<span>Strongly agree</span>
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</fieldset>
