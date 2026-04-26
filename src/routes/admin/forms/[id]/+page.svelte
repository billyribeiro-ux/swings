<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import { api } from '$lib/api/client';
	import FieldPalette from '$lib/components/admin/forms/FieldPalette.svelte';
	import FieldCanvas from '$lib/components/admin/forms/FieldCanvas.svelte';
	import FieldEditor from '$lib/components/admin/forms/FieldEditor.svelte';

	type FieldRow = {
		type: string;
		key: string;
		label?: string;
		placeholder?: string;
		help_text?: string;
		required?: boolean;
		options?: Array<{ value: string; label: string }>;
	};

	type FormDef = {
		id: string;
		slug: string;
		name: string;
		schema: FieldRow[];
		logic: unknown[];
		version: number;
	};

	const id = $derived(page.params.id ?? '');

	let formDef = $state<FormDef | null>(null);
	let schema = $state<FieldRow[]>([]);
	let selected = $state<number | null>(null);
	let saving = $state(false);
	let dirty = $state(false);
	let err = $state<string | null>(null);

	onMount(async () => {
		try {
			formDef = await api.get<FormDef>(`/admin/forms/${id}`);
			schema = Array.isArray(formDef?.schema) ? formDef.schema : [];
		} catch (e) {
			err = e instanceof Error ? e.message : 'Failed to load form.';
		}
	});

	function nextKey(type: string, taken: Set<string>): string {
		const base = type.replace(/_/g, '-');
		let i = 1;
		while (taken.has(`${base}-${i}`)) i++;
		return `${base}-${i}`;
	}

	function add(type: string, atIndex: number = schema.length) {
		const taken = new Set(schema.map((r) => r.key));
		const key = nextKey(type, taken);
		const row: FieldRow = { type, key, label: '' };
		if (['select', 'multi_select', 'radio', 'ranking'].includes(type)) {
			row.options = [
				{ value: 'opt-1', label: 'Option 1' },
				{ value: 'opt-2', label: 'Option 2' }
			];
		}
		const next = schema.slice();
		next.splice(atIndex, 0, row);
		schema = next;
		selected = atIndex;
		dirty = true;
	}

	function move(from: number, to: number) {
		const next = schema.slice();
		const [item] = next.splice(from, 1);
		if (!item) return;
		next.splice(to, 0, item);
		schema = next;
		selected = to;
		dirty = true;
	}

	function remove(index: number) {
		schema = schema.filter((_, i) => i !== index);
		selected = null;
		dirty = true;
	}

	function update(next: FieldRow) {
		if (selected === null) return;
		const arr = schema.slice();
		arr[selected] = next;
		schema = arr;
		dirty = true;
	}

	async function save() {
		if (!formDef) return;
		saving = true;
		err = null;
		try {
			await api.post(`/admin/forms/${id}/versions`, { schema, logic: [] });
			dirty = false;
		} catch (e) {
			err = e instanceof Error ? e.message : 'Save failed.';
		} finally {
			saving = false;
		}
	}

	const selectedField = $derived(selected === null ? null : (schema[selected] ?? null));
</script>

<svelte:head>
	<title>{formDef ? formDef.name : 'Form'} · Builder</title>
</svelte:head>

<header class="builder__header">
	<div>
		<a class="back" href="/admin/forms">← Forms</a>
		<h1 class="builder__title">
			{formDef ? formDef.name : 'Loading…'}
			{#if dirty}<span class="dirty">●</span>{/if}
		</h1>
	</div>
	<div class="builder__actions">
		<a class="btn" href="/admin/forms/{id}/preview"><EyeIcon size={16} />Preview</a>
		<button class="btn btn--primary" type="button" onclick={save} disabled={saving || !dirty}>
			<FloppyDiskIcon size={16} />{saving ? 'Saving…' : 'Save version'}
		</button>
	</div>
</header>

{#if err}<p class="error">{err}</p>{/if}

<div class="builder">
	<FieldPalette onAdd={(t) => add(t)} />
	<FieldCanvas
		{schema}
		selectedIndex={selected}
		onSelect={(i) => (selected = i)}
		onDelete={remove}
		onMove={move}
		onAdd={add}
	/>
	<FieldEditor field={selectedField} onChange={update} />
</div>

<style>
	.builder__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-block: var(--space-3);
		border-block-end: 1px solid var(--color-border);
		margin-block-end: var(--space-3);
	}
	.builder__title {
		font-size: var(--font-size-xl);
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}
	.dirty {
		color: var(--color-warning);
		font-size: 1.5em;
		line-height: 1;
	}
	.back {
		display: block;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		text-decoration: none;
	}
	.back:hover {
		color: var(--color-text);
	}
	.builder__actions {
		display: flex;
		gap: var(--space-2);
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-3);
		border-radius: var(--radius-sm);
		font-size: var(--font-size-sm);
		cursor: pointer;
		border: 1px solid var(--color-border);
		background: var(--color-surface-2);
		color: var(--color-text);
		text-decoration: none;
	}
	.btn--primary {
		background: var(--color-accent);
		color: var(--color-on-accent);
		border-color: transparent;
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.builder {
		display: grid;
		grid-template-columns: minmax(220px, 280px) minmax(0, 1fr) minmax(280px, 320px);
		min-block-size: calc(100vh - 12rem);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		overflow: hidden;
		background: var(--color-surface-1);
	}
	@media (max-width: 1024px) {
		.builder {
			grid-template-columns: 1fr;
		}
	}
	.error {
		color: var(--color-danger);
		font-size: var(--font-size-sm);
		padding: var(--space-2);
	}
</style>
