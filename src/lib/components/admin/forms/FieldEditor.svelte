<script lang="ts">
	type FieldRow = {
		type: string;
		key: string;
		label?: string;
		placeholder?: string;
		help_text?: string;
		required?: boolean;
		options?: Array<{ value: string; label: string }>;
	};

	type Props = {
		field: FieldRow | null;
		onChange: (next: FieldRow) => void;
	};
	const { field, onChange }: Props = $props();

	function patch(p: Partial<FieldRow>) {
		if (!field) return;
		onChange({ ...field, ...p });
	}

	function patchOption(idx: number, p: Partial<{ value: string; label: string }>) {
		if (!field?.options) return;
		const next = field.options.slice();
		const cur = next[idx];
		if (!cur) return;
		next[idx] = { ...cur, ...p };
		patch({ options: next });
	}

	function addOption() {
		const next = (field?.options ?? []).slice();
		next.push({ value: `opt-${next.length + 1}`, label: `Option ${next.length + 1}` });
		patch({ options: next });
	}

	function removeOption(idx: number) {
		if (!field?.options) return;
		patch({ options: field.options.filter((_, i) => i !== idx) });
	}

	const supportsOptions = $derived(
		field !== null && ['select', 'multi_select', 'radio', 'ranking'].includes(field.type)
	);
</script>

<aside class="editor" aria-label="Field properties">
	{#if field === null}
		<p class="editor__empty">Select a field to edit its properties.</p>
	{:else}
		<header class="editor__header">
			<span class="editor__type">{field.type}</span>
			<h2 class="editor__title">Properties</h2>
		</header>

		<div class="editor__group">
			<label class="editor__label" for="ed-key">Key</label>
			<input
				id="ed-key"
				class="editor__input editor__input--mono"
				type="text"
				value={field.key}
				oninput={(e) => patch({ key: e.currentTarget.value })}
			/>
			<small class="editor__hint"
				>Stable identifier; never rename after collecting submissions.</small
			>
		</div>

		<div class="editor__group">
			<label class="editor__label" for="ed-label">Label</label>
			<input
				id="ed-label"
				class="editor__input"
				type="text"
				value={field.label ?? ''}
				oninput={(e) => patch({ label: e.currentTarget.value })}
			/>
		</div>

		<div class="editor__group">
			<label class="editor__label" for="ed-ph">Placeholder</label>
			<input
				id="ed-ph"
				class="editor__input"
				type="text"
				value={field.placeholder ?? ''}
				oninput={(e) => patch({ placeholder: e.currentTarget.value })}
			/>
		</div>

		<div class="editor__group">
			<label class="editor__label" for="ed-help">Help text</label>
			<textarea
				id="ed-help"
				class="editor__input editor__input--multiline"
				rows="2"
				value={field.help_text ?? ''}
				oninput={(e) => patch({ help_text: e.currentTarget.value })}
			></textarea>
		</div>

		<div class="editor__group editor__group--inline">
			<input
				id="ed-required"
				type="checkbox"
				checked={field.required ?? false}
				onchange={(e) => patch({ required: e.currentTarget.checked })}
			/>
			<label class="editor__label editor__label--inline" for="ed-required">Required</label>
		</div>

		{#if supportsOptions}
			<div class="editor__group">
				<div class="editor__row">
					<span class="editor__label">Options</span>
					<button type="button" class="editor__add" onclick={addOption}>+ Add</button>
				</div>
				{#each field.options ?? [] as opt, idx (idx)}
					<div class="editor__option">
						<input
							class="editor__input editor__input--small editor__input--mono"
							type="text"
							value={opt.value}
							oninput={(e) => patchOption(idx, { value: e.currentTarget.value })}
							aria-label="Option value"
						/>
						<input
							class="editor__input editor__input--small"
							type="text"
							value={opt.label}
							oninput={(e) => patchOption(idx, { label: e.currentTarget.value })}
							aria-label="Option label"
						/>
						<button
							type="button"
							class="editor__remove"
							onclick={() => removeOption(idx)}
							aria-label="Remove option">×</button
						>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</aside>

<style>
	.editor {
		padding: var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		border-inline-start: 1px solid var(--color-border);
		min-block-size: 100%;
		background: var(--color-surface-1);
	}
	.editor__empty {
		color: var(--color-text-muted);
		text-align: center;
		padding: var(--space-6);
	}
	.editor__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.editor__type {
		font-size: var(--font-size-2xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 2px var(--space-2);
		background: var(--color-surface-3);
		border-radius: var(--radius-sm);
	}
	.editor__title {
		font-size: var(--font-size-sm);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.editor__group {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.editor__group--inline {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
	}
	.editor__row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.editor__label {
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.editor__label--inline {
		text-transform: none;
		letter-spacing: normal;
	}
	.editor__input {
		padding: var(--space-1) var(--space-2);
		font-size: var(--font-size-sm);
		background: var(--color-surface-2);
		color: var(--color-text);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
	}
	.editor__input:focus-visible {
		outline: 2px solid var(--color-focus-ring);
		outline-offset: 1px;
	}
	.editor__input--mono {
		font-family: var(--font-mono);
	}
	.editor__input--multiline {
		resize: vertical;
		min-block-size: 4lh;
	}
	.editor__input--small {
		padding: 2px var(--space-1);
	}
	.editor__hint {
		font-size: var(--font-size-2xs);
		color: var(--color-text-muted);
	}
	.editor__option {
		display: grid;
		grid-template-columns: 1fr 1fr auto;
		gap: var(--space-1);
		align-items: center;
	}
	.editor__add,
	.editor__remove {
		background: transparent;
		border: 1px solid var(--color-border);
		color: var(--color-text);
		padding: 2px var(--space-1);
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: var(--font-size-sm);
	}
	.editor__add:hover,
	.editor__remove:hover {
		background: var(--color-surface-3);
	}
</style>
