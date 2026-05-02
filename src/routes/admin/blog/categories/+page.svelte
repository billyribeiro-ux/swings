<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { BlogCategory } from '$lib/api/types';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import { toast } from '$lib/stores/toast.svelte';

	let categories: BlogCategory[] = $state([]);
	let loading = $state(true);
	let newName = $state('');
	let newDescription = $state('');
	let editingId: string | null = $state(null);
	let editName = $state('');
	let editDescription = $state('');

	onMount(loadCategories);

	async function loadCategories() {
		loading = true;
		try {
			categories = await api.get<BlogCategory[]>('/api/admin/blog/categories');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to load categories');
		} finally {
			loading = false;
		}
	}

	async function addCategory() {
		if (!newName.trim()) return;
		try {
			const cat = await api.post<BlogCategory>('/api/admin/blog/categories', {
				name: newName,
				description: newDescription || undefined
			});
			categories = [...categories, cat];
			newName = '';
			newDescription = '';
			toast.success('Category created');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to create category');
		}
	}

	function startEdit(cat: BlogCategory) {
		editingId = cat.id;
		editName = cat.name;
		editDescription = cat.description || '';
	}

	async function saveEdit() {
		if (!editingId || !editName.trim()) return;
		try {
			const updated = await api.put<BlogCategory>(`/api/admin/blog/categories/${editingId}`, {
				name: editName,
				description: editDescription || undefined
			});
			categories = categories.map((c) => (c.id === editingId ? updated : c));
			editingId = null;
			toast.success('Category updated');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to update category');
		}
	}

	async function deleteCategory(id: string) {
		const ok = await confirmDialog({
			title: 'Delete this category?',
			message: 'Posts assigned to this category will be moved to Uncategorized.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.delete(`/api/admin/blog/categories/${id}`);
			categories = categories.filter((c) => c.id !== id);
			toast.success('Category deleted');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Failed to delete category');
		}
	}
</script>

<svelte:head>
	<title>Categories — Admin</title>
</svelte:head>

<div class="cats-admin">
	<h1>Blog Categories</h1>

	<div class="cats-admin__layout">
		<!-- Add form -->
		<div class="cats-admin__form">
			<h3>Add New Category</h3>
			<label class="field">
				<span>Name</span>
				<input
					id="new-category-name"
					name="category-name"
					type="text"
					bind:value={newName}
					placeholder="Category name"
				/>
			</label>
			<label class="field">
				<span>Description</span>
				<textarea
					id="new-category-description"
					name="category-description"
					bind:value={newDescription}
					placeholder="Optional description"
					rows="3"
				></textarea>
			</label>
			<button class="btn-add" onclick={addCategory}>Add Category</button>
		</div>

		<!-- List -->
		<div class="cats-admin__list">
			{#if loading}
				<p class="empty">Loading...</p>
			{:else if categories.length === 0}
				<p class="empty">No categories yet.</p>
			{:else}
				<table class="cats-table">
					<thead>
						<tr>
							<th>Name</th>
							<th>Slug</th>
							<th>Description</th>
							<th>Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each categories as cat (cat.id)}
							<tr>
								{#if editingId === cat.id}
									<td>
										<input
											id="edit-cat-{cat.id}-name"
											name="edit-category-name"
											type="text"
											class="edit-input"
											bind:value={editName}
										/>
									</td>
									<td class="slug-cell">{cat.slug}</td>
									<td>
										<input
											id="edit-cat-{cat.id}-description"
											name="edit-category-description"
											type="text"
											class="edit-input"
											bind:value={editDescription}
										/>
									</td>
									<td class="actions-cell">
										<button class="action-link" onclick={saveEdit}>Save</button>
										<button
											class="action-link"
											onclick={() => (editingId = null)}>Cancel</button
										>
									</td>
								{:else}
									<td class="name-cell">{cat.name}</td>
									<td class="slug-cell">{cat.slug}</td>
									<td class="desc-cell">{cat.description || '—'}</td>
									<td class="actions-cell">
										<button class="action-link" onclick={() => startEdit(cat)}
											>Edit</button
										>
										<button
											class="action-link action-link--danger"
											onclick={() => deleteCategory(cat.id)}>Delete</button
										>
									</td>
								{/if}
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	</div>
</div>

<style>
	.cats-admin h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #fff;
		margin: 0 0 1.5rem;
	}

	.cats-admin__layout {
		display: grid;
		grid-template-columns: 18rem 1fr;
		gap: 2rem;
		align-items: start;
	}

	.cats-admin__form {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 0.5rem;
		padding: 1.25rem;
	}

	.cats-admin__form h3 {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--color-grey-200, #e2e8f0);
		margin: 0 0 1rem;
	}

	.field {
		display: block;
		margin-bottom: 0.75rem;
	}

	.field span {
		display: block;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-grey-400, #64748b);
		margin-bottom: 0.25rem;
	}

	.field input,
	.field textarea {
		width: 100%;
		padding: 0.4rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.2);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
		font-family: inherit;
	}

	.field input:focus,
	.field textarea:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.btn-add {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-size: 0.8rem;
		font-weight: 700;
		cursor: pointer;
	}

	.btn-add:hover {
		opacity: 0.9;
	}

	.empty {
		color: var(--color-grey-400, #64748b);
		text-align: center;
		padding: 2rem;
	}

	.cats-table {
		width: 100%;
		border-collapse: collapse;
	}

	.cats-table th {
		text-align: left;
		padding: 0.6rem 0.75rem;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-400, #64748b);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.cats-table td {
		padding: 0.6rem 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		font-size: 0.85rem;
		color: var(--color-grey-200, #e2e8f0);
	}

	.name-cell {
		font-weight: 600;
	}
	.slug-cell {
		color: var(--color-grey-400, #64748b);
		font-size: 0.8rem;
	}
	.desc-cell {
		color: var(--color-grey-400, #64748b);
		font-size: 0.8rem;
	}
	.actions-cell {
		white-space: nowrap;
	}

	.edit-input {
		width: 100%;
		padding: 0.3rem 0.4rem;
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.3);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
	}

	.action-link {
		border: none;
		background: none;
		color: var(--color-teal-light, #15c5d1);
		font-size: 0.8rem;
		cursor: pointer;
		padding: 0;
		margin-right: 0.75rem;
	}

	.action-link:hover {
		text-decoration: underline;
	}
	.action-link--danger {
		color: #ef4444;
	}

	@media (max-width: 768px) {
		.cats-admin__layout {
			grid-template-columns: 1fr;
		}
	}
</style>
