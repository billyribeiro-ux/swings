<!--
  FORM-10: Computed field. Evaluates a simple formula against sibling field
  values in `data`; the user cannot edit this control directly (it's a live
  readout). validate.ts treats calculation fields as optional; the server
  recomputes authoritatively before persist.

  Only a safe subset is supported client-side:
  - Addition, subtraction, multiplication, division (+, -, *, /)
  - Field references by key (wrapped in braces, e.g. `{qty} * {price}`)
  - Numeric literals

  Anything else evaluates to an empty display; the backend is the source
  of truth for arbitrary logic.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, data, error, onChange }: FieldProps = $props();
	const c = $derived(field as Extract<FieldSchema, { type: 'calculation' }>);
	const controlId = $derived(`form-field-${field.key}`);

	const computed = $derived.by(() => {
		try {
			return evalFormula(c.formula, data);
		} catch {
			return NaN;
		}
	});

	$effect(() => {
		if (Number.isFinite(computed)) {
			onChange(field.key, computed);
		}
	});

	function evalFormula(formula: string, d: Readonly<Record<string, unknown>>): number {
		// Substitute `{key}` with the numeric value.
		const substituted = formula.replace(/\{([a-zA-Z0-9_]+)\}/g, (_, k: string) => {
			const v = d[k];
			if (typeof v === 'number' && Number.isFinite(v)) return String(v);
			if (typeof v === 'string' && v.trim().length > 0) {
				const n = Number(v);
				if (Number.isFinite(n)) return String(n);
			}
			return '0';
		});
		// Allow only digits, ops, dots, parens, spaces.
		if (!/^[\d+\-*/().\s]+$/.test(substituted)) return NaN;
		// Function(substituted) is safe only because of the above gate.
		return Function(`"use strict"; return (${substituted});`)() as number;
	}
</script>

<FieldFrame
	{controlId}
	label={field.label ?? field.key}
	helpText={field.helpText}
	{error}
	required={false}
>
	{#snippet children({ describedBy })}
		<output
			id={controlId}
			class="fm-input fm-calc"
			for={field.key}
			aria-describedby={describedBy}
			aria-live="polite">{Number.isFinite(computed) ? computed : '—'}</output
		>
	{/snippet}
</FieldFrame>
