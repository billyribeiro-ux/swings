<!--
  FORM-10: Single-question quiz field. Scoring happens server-side by
  comparing the submitted value against `correct_value`; the renderer
  simply treats this as a radio group.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const q = $derived(field as Extract<FieldSchema, { type: 'quiz' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);
</script>

<fieldset
	class="fm-field fm-field--group fm-field--quiz"
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
	<p class="fm-quiz__question">{q.question}</p>
	{#if field.helpText}
		<p id={helpId} class="fm-field__help">{field.helpText}</p>
	{/if}
	<div class="fm-choices">
		{#each q.options as opt (opt.value)}
			<label class="fm-check">
				<input
					type="radio"
					class="fm-check__input"
					name={field.key}
					value={opt.value}
					checked={current === opt.value}
					disabled={disabled || (opt.disabled ?? false)}
					onchange={() => onChange(field.key, opt.value)}
				/>
				<span class="fm-check__label">{opt.label}</span>
			</label>
		{/each}
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</fieldset>
