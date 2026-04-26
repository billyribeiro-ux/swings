<script lang="ts">
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import ArrowUpIcon from 'phosphor-svelte/lib/ArrowUpIcon';
	import ArrowDownIcon from 'phosphor-svelte/lib/ArrowDownIcon';

	type FieldRow = { type: string; key: string; label?: string };

	type Props = {
		schema: FieldRow[];
		selectedIndex: number | null;
		onSelect: (index: number) => void;
		onDelete: (index: number) => void;
		onMove: (from: number, to: number) => void;
		onAdd: (type: string, atIndex: number) => void;
	};
	const { schema, selectedIndex, onSelect, onDelete, onMove, onAdd }: Props = $props();

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
	}
	function handleDrop(e: DragEvent, atIndex: number) {
		e.preventDefault();
		const type = e.dataTransfer?.getData('text/x-form-field');
		if (type) onAdd(type, atIndex);
	}
</script>

<section
	class="canvas"
	aria-label="Form canvas"
	ondragover={handleDragOver}
	ondrop={(e) => handleDrop(e, schema.length)}
>
	{#if schema.length === 0}
		<p class="canvas__empty">
			Drag a field from the palette, or click any field to add it to the form.
		</p>
	{/if}

	<ul class="canvas__list">
		{#each schema as row, i (row.key + i)}
			<li
				class="canvas__row"
				class:canvas__row--selected={selectedIndex === i}
				ondragover={handleDragOver}
				ondrop={(e) => handleDrop(e, i)}
			>
				<button
					type="button"
					class="canvas__row-body"
					onclick={() => onSelect(i)}
					aria-pressed={selectedIndex === i}
				>
					<span class="canvas__row-type">{row.type}</span>
					<span class="canvas__row-key">{row.key}</span>
					{#if row.label}
						<span class="canvas__row-label">{row.label}</span>
					{/if}
				</button>
				<div class="canvas__row-actions">
					<button
						type="button"
						class="icon-btn"
						aria-label="Move up"
						disabled={i === 0}
						onclick={() => onMove(i, i - 1)}
					>
						<ArrowUpIcon size={14} />
					</button>
					<button
						type="button"
						class="icon-btn"
						aria-label="Move down"
						disabled={i === schema.length - 1}
						onclick={() => onMove(i, i + 1)}
					>
						<ArrowDownIcon size={14} />
					</button>
					<button
						type="button"
						class="icon-btn icon-btn--danger"
						aria-label="Delete field"
						onclick={() => onDelete(i)}
					>
						<TrashIcon size={14} />
					</button>
				</div>
			</li>
		{/each}
	</ul>
</section>

<style>
	.canvas {
		padding: var(--space-4);
		min-block-size: 100%;
		background: var(--color-surface-1);
	}
	.canvas__empty {
		padding: var(--space-6);
		text-align: center;
		color: var(--color-text-muted);
		border: 2px dashed var(--color-border);
		border-radius: var(--radius-md);
	}
	.canvas__list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: grid;
		gap: var(--space-2);
	}
	.canvas__row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2);
		background: var(--color-surface-2);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		transition:
			border-color 120ms ease,
			box-shadow 120ms ease;
	}
	.canvas__row--selected {
		border-color: var(--color-accent);
		box-shadow: 0 0 0 2px var(--color-accent-soft);
	}
	.canvas__row-body {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-3);
		background: transparent;
		border: 0;
		padding: var(--space-1) var(--space-2);
		text-align: start;
		cursor: pointer;
	}
	.canvas__row-body:focus-visible {
		outline: 2px solid var(--color-focus-ring);
		outline-offset: 2px;
		border-radius: var(--radius-sm);
	}
	.canvas__row-type {
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		min-inline-size: 8ch;
	}
	.canvas__row-key {
		font-family: var(--font-mono);
		font-size: var(--font-size-sm);
	}
	.canvas__row-label {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.canvas__row-actions {
		display: flex;
		gap: var(--space-1);
	}
	.icon-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		inline-size: 28px;
		block-size: 28px;
		padding: 0;
		background: transparent;
		color: var(--color-text-muted);
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		cursor: pointer;
	}
	.icon-btn:hover {
		background: var(--color-surface-3);
		color: var(--color-text);
	}
	.icon-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.icon-btn:focus-visible {
		outline: 2px solid var(--color-focus-ring);
		outline-offset: 1px;
	}
	.icon-btn--danger:hover {
		color: var(--color-danger);
	}
</style>
