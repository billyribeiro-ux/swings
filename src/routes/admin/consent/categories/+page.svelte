<!--
  CONSENT-07 — category CRUD.

  `necessary` is protected by the backend (BadRequest on create/update); the
  UI renders the row read-only so admins can see it's present without
  accidentally re-submitting.

  Drag-to-reorder: native HTML5 drag API sits on top of `sort_order`. When
  the admin drops a row we compute a new sort_order and PUT it back. The
  server-side key is the stable identifier (categories cannot be renamed
  after production), so reordering by dragging is safe.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog } from '$lib/components/shared';
	import {
		listCategories,
		createCategory,
		updateCategory,
		type AdminCategory,
		type CategoryUpsertBody
	} from '$lib/api/admin-consent';

	let categories = $state<AdminCategory[]>([]);
	let loading = $state(true);
	let errorMsg = $state<string | null>(null);

	let editorOpen = $state(false);
	let editingKey = $state<string | null>(null);
	let keyInput = $state('');
	let labelInput = $state('');
	let descriptionInput = $state('');
	let isRequired = $state(false);
	let sortOrder = $state(0);

	async function refresh() {
		loading = true;
		errorMsg = null;
		try {
			categories = await listCategories();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	onMount(() => void refresh());

	function openCreate() {
		editingKey = null;
		keyInput = '';
		labelInput = '';
		descriptionInput = '';
		isRequired = false;
		sortOrder = Math.max(0, ...categories.map((c) => c.sort_order)) + 10;
		editorOpen = true;
	}

	function openEdit(c: AdminCategory) {
		editingKey = c.key;
		keyInput = c.key;
		labelInput = c.label;
		descriptionInput = c.description;
		isRequired = c.is_required;
		sortOrder = c.sort_order;
		editorOpen = true;
	}

	async function save() {
		const body: CategoryUpsertBody = {
			key: keyInput,
			label: labelInput,
			description: descriptionInput,
			is_required: isRequired,
			sort_order: sortOrder
		};
		try {
			if (editingKey) {
				await updateCategory(editingKey, body);
			} else {
				await createCategory(body);
			}
			editorOpen = false;
			await refresh();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		}
	}

	// ── Drag-to-reorder ────────────────────────────────────────────────
	let dragKey = $state<string | null>(null);

	function onDragStart(key: string) {
		if (key === 'necessary') return;
		dragKey = key;
	}

	async function onDrop(targetKey: string) {
		if (!dragKey || dragKey === targetKey || targetKey === 'necessary') {
			dragKey = null;
			return;
		}
		const src = categories.find((c) => c.key === dragKey);
		const tgt = categories.find((c) => c.key === targetKey);
		if (!src || !tgt) {
			dragKey = null;
			return;
		}
		const body: CategoryUpsertBody = {
			key: src.key,
			label: src.label,
			description: src.description,
			is_required: src.is_required,
			sort_order: tgt.sort_order
		};
		try {
			await updateCategory(src.key, body);
			await refresh();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		}
		dragKey = null;
	}
</script>

<svelte:head>
	<title>Categories · Consent · Admin</title>
</svelte:head>

<header class="head">
	<h1>Consent categories</h1>
	<Button variant="primary" size="md" onclick={openCreate}>New category</Button>
</header>

{#if errorMsg}<div class="error">{errorMsg}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else}
	<table class="table">
		<thead>
			<tr>
				<th></th>
				<th>Key</th>
				<th>Label</th>
				<th>Required</th>
				<th>Sort</th>
				<th></th>
			</tr>
		</thead>
		<tbody>
			{#each categories as c (c.key)}
				{@const draggable = c.key !== 'necessary'}
				<tr
					draggable={draggable ? 'true' : 'false'}
					ondragstart={() => onDragStart(c.key)}
					ondragover={(e) => e.preventDefault()}
					ondrop={() => onDrop(c.key)}
				>
					<td class="handle" aria-hidden="true">{draggable ? '⋮⋮' : ''}</td>
					<td><code>{c.key}</code></td>
					<td>{c.label}</td>
					<td>{c.is_required ? 'yes' : 'no'}</td>
					<td>{c.sort_order}</td>
					<td>
						{#if c.key === 'necessary'}
							<span class="muted">protected</span>
						{:else}
							<Button variant="tertiary" size="sm" onclick={() => openEdit(c)}>Edit</Button>
						{/if}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}

<Dialog
	open={editorOpen}
	onclose={() => (editorOpen = false)}
	title={editingKey ? 'Edit category' : 'New category'}
	size="md"
>
	<div class="form">
		<label class="field">
			<span>Key (stable identifier)</span>
			<input type="text" bind:value={keyInput} disabled={!!editingKey} />
		</label>
		<label class="field">
			<span>Label</span>
			<input type="text" bind:value={labelInput} />
		</label>
		<label class="field">
			<span>Description</span>
			<textarea rows="3" bind:value={descriptionInput}></textarea>
		</label>
		<div class="row">
			<label class="field field--toggle">
				<input type="checkbox" bind:checked={isRequired} />
				<span>Required (always granted)</span>
			</label>
			<label class="field">
				<span>Sort order</span>
				<input type="number" bind:value={sortOrder} />
			</label>
		</div>
	</div>
	{#snippet footer()}
		<Button variant="tertiary" onclick={() => (editorOpen = false)}>Cancel</Button>
		<Button variant="primary" onclick={save}>Save</Button>
	{/snippet}
</Dialog>

<style>
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		margin-block-end: var(--space-5);
	}
	.head h1 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		border: 1px solid var(--surface-border-subtle);
		background-color: var(--surface-bg-canvas);
		margin-block-end: var(--space-4);
	}
	.muted {
		color: var(--surface-fg-muted);
	}
	.table {
		inline-size: 100%;
		border-collapse: collapse;
	}
	.table th,
	.table td {
		padding: var(--space-3);
		border-block-end: 1px solid var(--surface-border-subtle);
		font-size: var(--fs-sm);
		text-align: start;
	}
	.handle {
		inline-size: 1.5rem;
		color: var(--surface-fg-muted);
		cursor: grab;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.row {
		display: flex;
		gap: var(--space-4);
		flex-wrap: wrap;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
		flex: 1 1 12rem;
		font-size: var(--fs-sm);
	}
	.field--toggle {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
	}
	input[type='text'],
	input[type='number'],
	textarea {
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		font: inherit;
	}
</style>
