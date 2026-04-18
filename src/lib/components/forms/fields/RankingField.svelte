<!--
  FORM-10: Drag-to-rank ordering. Uses the native HTML5 Sortable API via
  `draggable="true"` so we avoid a runtime dep. Keyboard fallback: each
  item exposes up/down buttons that move it within the order.

  The stored value is an array of option `value`s in the user's preferred
  order.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';
	import CaretUp from 'phosphor-svelte/lib/CaretUp';
	import CaretDown from 'phosphor-svelte/lib/CaretDown';
	import DotsSixVertical from 'phosphor-svelte/lib/DotsSixVertical';

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const r = $derived(field as Extract<FieldSchema, { type: 'ranking' }>);

	// Union of schema-order + current order; unknown values are dropped.
	const order = $derived.by(() => {
		const known = new Set(r.options.map((o) => o.value));
		const fromValue = Array.isArray(value) ? (value as unknown[]).filter((v): v is string => typeof v === 'string' && known.has(v)) : [];
		const seen = new Set(fromValue);
		const fallback = r.options.map((o) => o.value).filter((v) => !seen.has(v));
		return [...fromValue, ...fallback];
	});

	const controlId = $derived(`form-field-${field.key}`);
	const errorId = $derived(error ? `${controlId}-err` : undefined);

	let dragFrom = $state<number | null>(null);

	function labelFor(v: string): string {
		return r.options.find((o) => o.value === v)?.label ?? v;
	}

	function reorder(from: number, to: number) {
		if (from === to || from < 0 || to < 0 || from >= order.length || to >= order.length) return;
		const next = [...order];
		const [moved] = next.splice(from, 1);
		next.splice(to, 0, moved);
		onChange(field.key, next);
	}

	function handleDragStart(i: number) {
		dragFrom = i;
	}
	function handleDragOver(e: DragEvent) {
		e.preventDefault();
	}
	function handleDrop(i: number) {
		if (dragFrom === null) return;
		reorder(dragFrom, i);
		dragFrom = null;
	}
	function moveUp(i: number) {
		reorder(i, i - 1);
	}
	function moveDown(i: number) {
		reorder(i, i + 1);
	}
</script>

<div class="fm-field fm-field--ranking" class:fm-field--invalid={!!error}>
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
		<p class="fm-field__help">{field.helpText}</p>
	{/if}
	<ol id={controlId} class="fm-ranking">
		{#each order as v, i (v)}
			<li
				class="fm-ranking__item"
				draggable={!disabled}
				ondragstart={() => handleDragStart(i)}
				ondragover={handleDragOver}
				ondrop={() => handleDrop(i)}
			>
				<DotsSixVertical size={18} />
				<span class="fm-ranking__label">{labelFor(v)}</span>
				<button
					type="button"
					class="fm-ranking__btn"
					onclick={() => moveUp(i)}
					disabled={disabled || i === 0}
					aria-label={`Move ${labelFor(v)} up`}
				>
					<CaretUp size={16} />
				</button>
				<button
					type="button"
					class="fm-ranking__btn"
					onclick={() => moveDown(i)}
					disabled={disabled || i === order.length - 1}
					aria-label={`Move ${labelFor(v)} down`}
				>
					<CaretDown size={16} />
				</button>
			</li>
		{/each}
	</ol>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</div>
