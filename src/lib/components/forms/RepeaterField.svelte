<!--
  FORM-10 (repeaters): variable-length row container.

  The repeater's value is an array of row objects; each row's key/value pairs
  match the configured sub-field schema. The component reuses `FormField`
  for every sub-field so behaviour + a11y stay consistent with the top-level
  renderer.

  v1 limits:
  - Min / max row counts (defaulted 0..=∞).
  - Drag reordering is a future enhancement; Up/Down buttons are provided.
  - Row-level validation flows through the shared validator by assembling
    synthetic `{ <parent.key>.<idx>.<sub.key> }` keys — but since the public
    renderer exposes the row data under the parent key, this is deferred
    to the server.

  Props are typed locally rather than in `types.ts` because the repeater is
  a composite control, not a FieldSchema variant.
-->
<script lang="ts" module>
	import type { FieldSchema, FieldSetter, FormDataMap } from './types.ts';

	export interface RepeaterProps {
		/** Parent field key under which the row array lives. */
		readonly fieldKey: string;
		readonly label: string;
		readonly helpText?: string;
		readonly required?: boolean;
		readonly min?: number;
		readonly max?: number;
		/** Sub-field schema declared once, rendered per row. */
		readonly subFields: readonly FieldSchema[];
		/** Row value array (from parent's data map). */
		readonly value: readonly Record<string, unknown>[];
		readonly disabled?: boolean;
		/** Per-row errors, keyed by "row.<idx>.<sub.key>". */
		readonly errors?: ReadonlyMap<string, string>;
		readonly onChange: FieldSetter;
	}
</script>

<script lang="ts">
	import FormField from './FormField.svelte';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';

	const {
		fieldKey,
		label,
		helpText,
		required = false,
		min = 0,
		max = Number.MAX_SAFE_INTEGER,
		subFields,
		value,
		disabled = false,
		errors,
		onChange
	}: RepeaterProps = $props();

	const controlId = $derived(`form-field-${fieldKey}`);
	const helpId = $derived(helpText ? `${controlId}-help` : undefined);
	const rows = $derived<readonly Record<string, unknown>[]>(value.length === 0 && min > 0 ? blankRows(min) : value);
	const canAdd = $derived(rows.length < max);
	const canRemove = $derived(rows.length > min);

	function blankRows(n: number): Record<string, unknown>[] {
		return Array.from({ length: n }, () => blankRow());
	}

	function blankRow(): Record<string, unknown> {
		const row: Record<string, unknown> = {};
		for (const sub of subFields) {
			row[sub.key] = '';
		}
		return row;
	}

	function updateRow(idx: number, subKey: string, v: unknown) {
		const next = rows.map((r, i) => (i === idx ? { ...r, [subKey]: v } : r));
		onChange(fieldKey, next);
	}

	function addRow() {
		if (!canAdd) return;
		onChange(fieldKey, [...rows, blankRow()]);
	}

	function removeRow(idx: number) {
		if (!canRemove) return;
		onChange(fieldKey, rows.filter((_, i) => i !== idx));
	}

	function moveRow(from: number, to: number) {
		if (from === to || to < 0 || to >= rows.length) return;
		const next = [...rows];
		const [moved] = next.splice(from, 1);
		next.splice(to, 0, moved);
		onChange(fieldKey, next);
	}

	function rowData(row: Record<string, unknown>): FormDataMap {
		return row as FormDataMap;
	}

	function rowSetter(idx: number): FieldSetter {
		return (key, v) => updateRow(idx, key, v);
	}

	function errorFor(idx: number, subKey: string): string | undefined {
		return errors?.get(`${fieldKey}.${idx}.${subKey}`);
	}
</script>

<fieldset class="fm-field fm-repeater" aria-describedby={helpId} id={controlId}>
	<legend class="fm-field__label">
		{label}
		{#if required}
			<span class="fm-field__required" aria-hidden="true">*</span>
			<span class="fm-field__sr">(required)</span>
		{/if}
	</legend>
	{#if helpText}
		<p id={helpId} class="fm-field__help">{helpText}</p>
	{/if}

	<ol class="fm-repeater__rows">
		{#each rows as row, i (i)}
			<li class="fm-repeater__row">
				<div class="fm-repeater__row-header">
					<span class="fm-repeater__row-index">#{i + 1}</span>
					<div class="fm-repeater__row-actions">
						<button
							type="button"
							class="fm-ranking__btn"
							onclick={() => moveRow(i, i - 1)}
							disabled={disabled || i === 0}
							aria-label={`Move row ${i + 1} up`}
						>
							<CaretUpIcon size={16} />
						</button>
						<button
							type="button"
							class="fm-ranking__btn"
							onclick={() => moveRow(i, i + 1)}
							disabled={disabled || i === rows.length - 1}
							aria-label={`Move row ${i + 1} down`}
						>
							<CaretDownIcon size={16} />
						</button>
						<button
							type="button"
							class="fm-ranking__btn fm-repeater__remove"
							onclick={() => removeRow(i)}
							disabled={disabled || !canRemove}
							aria-label={`Remove row ${i + 1}`}
						>
							<TrashIcon size={16} />
						</button>
					</div>
				</div>
				<div class="fm-repeater__row-fields">
					{#each subFields as sub (sub.key)}
						<FormField
							field={sub}
							value={row[sub.key]}
							data={rowData(row)}
							error={errorFor(i, sub.key)}
							{disabled}
							onChange={rowSetter(i)}
						/>
					{/each}
				</div>
			</li>
		{/each}
	</ol>

	{#if canAdd}
		<button
			type="button"
			class="fm-btn fm-btn--ghost fm-repeater__add"
			onclick={addRow}
			{disabled}
		>
			<PlusIcon size={18} />
			<span>Add row</span>
		</button>
	{/if}
</fieldset>
