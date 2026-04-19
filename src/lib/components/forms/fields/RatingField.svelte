<!--
  FORM-10: StarIcon rating. Implemented as a radio group styled as stars so
  keyboard + SR users get a first-class experience; visual fill is driven
  entirely by sibling selectors in forms.css.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';
	import StarIcon from 'phosphor-svelte/lib/StarIcon';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const r = $derived(field as Extract<FieldSchema, { type: 'rating' }>);
	const maxStars = $derived(r.max_stars ?? 5);
	const current = $derived(
		typeof value === 'number' && Number.isInteger(value) ? value : 0
	);
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);

	function handlePick(n: number) {
		onChange(field.key, n);
	}
</script>

<fieldset
	class="fm-field fm-field--group fm-field--rating"
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
	<div class="fm-rating" role="radiogroup" aria-label={field.label ?? field.key}>
		{#each Array.from({ length: maxStars }, (_, i) => i + 1) as n (n)}
			<label class="fm-rating__star" class:fm-rating__star--on={n <= current}>
				<input
					type="radio"
					name={field.key}
					value={n}
					class="fm-rating__input"
					checked={n === current}
					{disabled}
					onchange={() => handlePick(n)}
					aria-label={`${n} of ${maxStars}`}
				/>
				{#if n <= current}
					<StarIcon size={28} weight="fill" />
				{:else}
					<StarIcon size={28} weight="regular" />
				{/if}
			</label>
		{/each}
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</fieldset>
