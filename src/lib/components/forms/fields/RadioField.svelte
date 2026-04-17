<!--
  FORM-10: Single-choice radio group inside a `<fieldset>` + `<legend>` so
  assistive tech announces the group label; arrow-key navigation is native
  to grouped `input[type=radio]` sharing a `name`.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const rd = $derived(field as Extract<FieldSchema, { type: 'radio' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);

	function handleSelect(v: string) {
		onChange(field.key, v);
	}
</script>

<fieldset
	class="fm-field fm-field--group"
	class:fm-field--invalid={!!error}
	aria-describedby={describedBy}
	id={controlId}
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
	<div class="fm-choices">
		{#each rd.options as opt (opt.value)}
			<label class="fm-check">
				<input
					type="radio"
					class="fm-check__input"
					name={field.key}
					value={opt.value}
					checked={current === opt.value}
					disabled={disabled || (opt.disabled ?? false)}
					onchange={() => handleSelect(opt.value)}
				/>
				<span class="fm-check__label">{opt.label}</span>
			</label>
		{/each}
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</fieldset>
