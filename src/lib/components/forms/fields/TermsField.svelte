<!--
  FORM-10: Terms-of-service acceptance checkbox. Renders the terms URL
  as an external link (target="_blank" with noopener/noreferrer) so users
  can review without losing their form state.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const t = $derived(field as Extract<FieldSchema, { type: 'terms' }>);
	const checked = $derived(value === true);
	const controlId = $derived(`form-field-${field.key}`);
	const errorId = $derived(error ? `${controlId}-err` : undefined);

	function handleChange(e: Event) {
		const target = e.currentTarget as HTMLInputElement;
		onChange(field.key, target.checked);
	}
</script>

<div class="fm-field fm-field--consent" class:fm-field--invalid={!!error}>
	<label class="fm-check" for={controlId}>
		<input
			id={controlId}
			name={field.key}
			type="checkbox"
			class="fm-check__input"
			{checked}
			{disabled}
			aria-invalid={!!error}
			aria-required={field.required ?? false}
			aria-describedby={errorId}
			onchange={handleChange}
		/>
		<span class="fm-check__label">
			{field.label ?? 'I accept the'}
			<a href={t.terms_url} target="_blank" rel="noopener noreferrer">terms and conditions</a>
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
