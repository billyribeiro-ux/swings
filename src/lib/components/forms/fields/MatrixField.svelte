<!--
  FORM-10: Matrix of Likert responses. One radio group per row, columns
  reused as the shared option axis. Stored as `{ [row]: column_value }`.

  A11y: native `<table>` + `<th scope="col">` / `<th scope="row">` so SR
  users hear the row + column label when navigating.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const m = $derived(field as Extract<FieldSchema, { type: 'matrix' }>);
	const current = $derived<Record<string, string>>(toMap(value));
	const controlId = $derived(`form-field-${field.key}`);
	const errorId = $derived(error ? `${controlId}-err` : undefined);

	function toMap(v: unknown): Record<string, string> {
		if (v && typeof v === 'object' && !Array.isArray(v)) {
			const rec = v as Record<string, unknown>;
			const out: Record<string, string> = {};
			for (const [k, val] of Object.entries(rec)) {
				if (typeof val === 'string') out[k] = val;
			}
			return out;
		}
		return {};
	}

	function pick(row: string, col: string) {
		onChange(field.key, { ...current, [row]: col });
	}
</script>

<div class="fm-field fm-field--matrix" class:fm-field--invalid={!!error}>
	<div class="fm-field__label">
		<span>
			{field.label ?? field.key}
			{#if field.required}
				<span class="fm-field__required" aria-hidden="true">*</span>
				<span class="fm-field__sr">(required)</span>
			{/if}
		</span>
	</div>
	<div class="fm-matrix" role="region" aria-label={field.label ?? field.key}>
		<table class="fm-matrix__table">
			<thead>
				<tr>
					<th class="fm-matrix__corner" scope="col"></th>
					{#each m.columns as col (col)}
						<th scope="col" class="fm-matrix__col-head">{col}</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each m.rows as row (row)}
					<tr>
						<th scope="row" class="fm-matrix__row-head">{row}</th>
						{#each m.columns as col (col)}
							<td class="fm-matrix__cell">
								<label>
									<span class="fm-field__sr">{row}: {col}</span>
									<input
										type="radio"
										name={`${field.key}:${row}`}
										value={col}
										checked={current[row] === col}
										{disabled}
										onchange={() => pick(row, col)}
									/>
								</label>
							</td>
						{/each}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>
