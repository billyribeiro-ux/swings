<!--
  FORM-10: Rich-text editor. Uses a contenteditable host so we avoid pulling
  TipTap into every public form — the backend sanitises the HTML before
  persisting. For admin-heavy use cases the builder can swap in a full TipTap
  instance later.

  A11y: `role="textbox"` + `aria-multiline` + `aria-label`; character-count
  text lives under an `id` we advertise via `aria-describedby`.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const rt = $derived(field as Extract<FieldSchema, { type: 'rich_text' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);
	const countId = $derived(`${controlId}-count`);
	const describedBy = $derived(
		[helpId, errorId, countId].filter((s): s is string => typeof s === 'string').join(' ')
	);

	let hostEl = $state<HTMLDivElement | null>(null);
	const charCount = $derived(stripTags(current).length);

	function stripTags(html: string): string {
		return html.replace(/<[^>]+>/g, '').trim();
	}

	function handleInput() {
		if (!hostEl) return;
		onChange(field.key, hostEl.innerHTML);
	}

	$effect(() => {
		// Keep DOM in sync when external code replaces the value (e.g. resume).
		if (hostEl && hostEl.innerHTML !== current) {
			hostEl.innerHTML = current;
		}
	});
</script>

<div class="fm-field" class:fm-field--invalid={!!error}>
	<div class="fm-field__label">
		<label for={controlId}>
			{field.label ?? field.key}
			{#if field.required}
				<span class="fm-field__required" aria-hidden="true">*</span>
				<span class="fm-field__sr">(required)</span>
			{/if}
		</label>
	</div>
	{#if field.helpText}
		<p id={helpId} class="fm-field__help">{field.helpText}</p>
	{/if}
	<div
		bind:this={hostEl}
		id={controlId}
		class="fm-richtext"
		contenteditable={disabled ? 'false' : 'true'}
		role="textbox"
		aria-multiline="true"
		aria-describedby={describedBy}
		aria-invalid={!!error}
		aria-required={field.required ?? false}
		aria-label={field.label ?? field.key}
		oninput={handleInput}
		tabindex="0"
	></div>
	<p id={countId} class="fm-field__help">
		{charCount}
		{#if rt.max_length}/ {rt.max_length}{/if}
		 characters
	</p>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>
