<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { BlogTag } from '$lib/api/types';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	let tags: BlogTag[] = $state([]);
	let loading = $state(true);
	let newName = $state('');

	onMount(loadTags);

	async function loadTags() {
		loading = true;
		try {
			tags = await api.get<BlogTag[]>('/api/admin/blog/tags');
		} catch (e) {
			console.error('Failed to load tags', e);
		} finally {
			loading = false;
		}
	}

	async function addTag() {
		if (!newName.trim()) return;
		try {
			const tag = await api.post<BlogTag>('/api/admin/blog/tags', { name: newName });
			tags = [...tags, tag];
			newName = '';
		} catch (e) {
			console.error('Failed to create tag', e);
		}
	}

	async function deleteTag(id: string) {
		const ok = await confirmDialog({
			title: 'Delete this tag?',
			message: 'The tag will be removed from every post that currently uses it.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.delete(`/api/admin/blog/tags/${id}`);
			tags = tags.filter((t) => t.id !== id);
		} catch (e) {
			console.error('Failed to delete tag', e);
		}
	}
</script>

<svelte:head>
	<title>Tags — Admin</title>
</svelte:head>

<div class="tags-admin">
	<h1>Blog Tags</h1>

	<div class="tags-admin__layout">
		<div class="tags-admin__form">
			<h3>Add New Tag</h3>
			<div class="add-row">
				<input
					id="new-tag-name"
					name="tag-name"
					type="text"
					bind:value={newName}
					placeholder="Tag name"
					onkeydown={(e) => e.key === 'Enter' && addTag()}
				/>
				<button onclick={addTag}>Add</button>
			</div>
		</div>

		<div class="tags-admin__list">
			{#if loading}
				<p class="empty">Loading...</p>
			{:else if tags.length === 0}
				<p class="empty">No tags yet.</p>
			{:else}
				<table class="tags-table">
					<thead>
						<tr>
							<th>Name</th>
							<th>Slug</th>
							<th>Created</th>
							<th>Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each tags as tag (tag.id)}
							<tr>
								<td class="name-cell">{tag.name}</td>
								<td class="slug-cell">{tag.slug}</td>
								<td class="date-cell">
									{new Date(tag.created_at).toLocaleDateString('en-US', {
										month: 'short',
										day: 'numeric',
										year: 'numeric'
									})}
								</td>
								<td>
									<button
										class="action-link action-link--danger"
										onclick={() => deleteTag(tag.id)}>Delete</button
									>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</div>
	</div>
</div>

<style>
	.tags-admin h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #fff;
		margin: 0 0 1.5rem;
	}

	.tags-admin__layout {
		display: grid;
		grid-template-columns: 18rem 1fr;
		gap: 2rem;
		align-items: start;
	}

	.tags-admin__form {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 0.5rem;
		padding: 1.25rem;
	}

	.tags-admin__form h3 {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--color-grey-200, #e2e8f0);
		margin: 0 0 1rem;
	}

	.add-row {
		display: flex;
		gap: 0.5rem;
	}

	.add-row input {
		flex: 1;
		padding: 0.4rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.2);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
	}

	.add-row input:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.add-row button {
		padding: 0.4rem 1rem;
		border: none;
		border-radius: 0.25rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-size: 0.8rem;
		font-weight: 700;
		cursor: pointer;
	}

	.empty {
		color: var(--color-grey-400, #64748b);
		text-align: center;
		padding: 2rem;
	}

	.tags-table {
		width: 100%;
		border-collapse: collapse;
	}

	.tags-table th {
		text-align: left;
		padding: 0.6rem 0.75rem;
		font-size: 0.7rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-400, #64748b);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.tags-table td {
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
	.date-cell {
		color: var(--color-grey-400, #64748b);
		font-size: 0.8rem;
		white-space: nowrap;
	}

	.action-link {
		border: none;
		background: none;
		font-size: 0.8rem;
		cursor: pointer;
		padding: 0;
	}

	.action-link--danger {
		color: #ef4444;
	}
	.action-link--danger:hover {
		text-decoration: underline;
	}

	@media (max-width: 768px) {
		.tags-admin__layout {
			grid-template-columns: 1fr;
		}
	}
</style>
