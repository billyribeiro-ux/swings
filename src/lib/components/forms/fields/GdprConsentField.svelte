<!--
  FORM-10: GDPR consent checkbox. Per Art. 7(2) the consent text must be
  explicit + distinguishable, so it's rendered as a paragraph above the
  control — not inline as a label — and the checkbox label echoes the
  "I agree" affirmation.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const gdpr = $derived(field as Extract<FieldSchema, { type: 'gdpr_consent' }>);
	const checked = $derived(value === true);
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(`${controlId}-consent`);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ')
	);

	function handleChange(e: Event) {
		const target = e.currentTarget as HTMLInputElement;
		onChange(field.key, target.checked);
	}
</script>

<div class="fm-field fm-field--consent" class:fm-field--invalid={!!error}>
	<p id={helpId} class="fm-consent__text">{gdpr.consent_text}</p>
	<label class="fm-check" for={controlId}>
		<input
			id={controlId}
			name={field.key}
			type="checkbox"
			class="fm-check__input"
			{checked}
			{disabled}
			aria-describedby={describedBy}
			aria-invalid={!!error}
			aria-required={field.required ?? false}
			onchange={handleChange}
		/>
		<span class="fm-check__label">
			{field.label ?? 'I agree'}
			{#if field.required}
				<span class="fm-field__required" aria-hidden="true">*</span>
				<span class="fm-field__sr">(required)</span>
			{/if}
		</span>
	</label>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>
