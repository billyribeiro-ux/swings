<!--
  FORM-10: Multi-choice selector. Renders as a fieldset of checkboxes so
  assistive tech announces the group + per-option selection state; this also
  avoids native `<select multiple>`'s poor mobile UX.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const ms = $derived(field as Extract<FieldSchema, { type: 'multi_select' }>);
	const selected = $derived<readonly string[]>(Array.isArray(value) ? (value as string[]) : []);
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);

	function handleToggle(optValue: string, checked: boolean) {
		// Local-scope dedupe — the Set never escapes the function and the
		// array shipped via onChange is what the renderer consumes.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const next = new Set<string>(selected);
		if (checked) next.add(optValue);
		else next.delete(optValue);
		onChange(field.key, Array.from(next));
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
		{#each ms.options as opt (opt.value)}
			<label class="fm-check">
				<input
					type="checkbox"
					class="fm-check__input"
					value={opt.value}
					checked={selected.includes(opt.value)}
					disabled={disabled || (opt.disabled ?? false)}
					onchange={(e) =>
						handleToggle(opt.value, (e.currentTarget as HTMLInputElement).checked)}
				/>
				<span class="fm-check__label">{opt.label}</span>
			</label>
		{/each}
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</fieldset>
