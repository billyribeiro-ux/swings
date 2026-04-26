<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { MediaItem, PaginatedResponse } from '$lib/api/types';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	let items: MediaItem[] = $state([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let uploading = $state(false);
	let dragOver = $state(false);
	let selected: MediaItem | null = $state(null);
	let editTitle = $state('');
	let editAlt = $state('');
	let editCaption = $state('');
	let savingMeta = $state(false);
	let search = $state('');
	let viewMode: 'grid' | 'list' = $state('grid');

	const filteredItems = $derived(
		search.trim()
			? items.filter(
					(i) =>
						i.original_filename.toLowerCase().includes(search.toLowerCase()) ||
						(i.title || '').toLowerCase().includes(search.toLowerCase())
				)
			: items
	);

	onMount(loadMedia);

	async function loadMedia() {
		loading = true;
		try {
			const res = await api.get<PaginatedResponse<MediaItem>>(
				`/api/admin/blog/media?page=${page}&per_page=30`
			);
			items = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch (e) {
			console.error('Failed to load media', e);
		} finally {
			loading = false;
		}
	}

	async function uploadFiles(e: Event) {
		const input = e.target as HTMLInputElement;
		const files = input.files;
		if (!files) return;
		uploading = true;
		for (const file of files) {
			try {
				const formData = new FormData();
				formData.append('file', file);
				const media = await api.upload<MediaItem>('/api/admin/blog/media/upload', formData);
				items = [media, ...items];
				total += 1;
			} catch (err) {
				console.error('Upload failed for', file.name, err);
			}
		}
		uploading = false;
		input.value = '';
	}

	async function deleteItem(id: string) {
		const ok = await confirmDialog({
			title: 'Delete this media file?',
			message: 'The file will be permanently removed and any post that embeds it will lose the asset.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.delete(`/api/admin/blog/media/${id}`);
			items = items.filter((i) => i.id !== id);
			total -= 1;
			if (selected?.id === id) selected = null;
		} catch (e) {
			console.error('Failed to delete', e);
		}
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function selectItem(item: MediaItem) {
		selected = item;
		editTitle = item.title || '';
		editAlt = item.alt_text || '';
		editCaption = item.caption || '';
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		const files = e.dataTransfer?.files;
		if (!files) return;
		uploading = true;
		(async () => {
			for (const file of files) {
				try {
					const fd = new FormData();
					fd.append('file', file);
					const media = await api.upload<MediaItem>('/api/admin/blog/media/upload', fd);
					items = [media, ...items];
					total += 1;
				} catch (err) {
					console.error(err);
				}
			}
			uploading = false;
		})();
	}

	async function saveMetadata() {
		if (!selected) return;
		savingMeta = true;
		try {
			const updated = await api.put<MediaItem>(`/api/admin/blog/media/${selected.id}`, {
				title: editTitle || undefined,
				alt_text: editAlt || undefined,
				caption: editCaption || undefined
			});
			selected = updated;
			items = items.map((i) => (i.id === updated.id ? updated : i));
		} catch (e) {
			console.error('Failed to update metadata', e);
		} finally {
			savingMeta = false;
		}
	}

	function copyUrl(url: string) {
		navigator.clipboard.writeText(url);
	}
</script>

<svelte:head>
	<title>Media Library — Admin</title>
</svelte:head>

<div
	class="media-admin"
	class:media-admin--drag={dragOver}
	role="region"
	aria-label="Media Library"
	ondrop={handleDrop}
	ondragover={(e) => {
		e.preventDefault();
		dragOver = true;
	}}
	ondragleave={() => (dragOver = false)}
>
	<div class="media-admin__header">
		<h1>
			Media Library <span class="media-admin__count">{total} file{total !== 1 ? 's' : ''}</span>
		</h1>
		<div class="media-admin__header-actions">
			<input
				type="search"
				class="media-admin__search"
				bind:value={search}
				placeholder="Search files…"
			/>
			<div class="view-toggle">
				<button
					class:active={viewMode === 'grid'}
					onclick={() => (viewMode = 'grid')}
					title="Grid view">▦</button
				>
				<button
					class:active={viewMode === 'list'}
					onclick={() => (viewMode = 'list')}
					title="List view">≡</button
				>
			</div>
			<label class="btn-upload">
				{uploading ? 'Uploading…' : '↑ Upload'}
				<input
					id="media-upload-input"
					name="file"
					type="file"
					accept="image/*,application/pdf"
					multiple
					hidden
					onchange={uploadFiles}
				/>
			</label>
		</div>
	</div>

	{#if dragOver}
		<div class="media-admin__drop-hint">Drop files to upload</div>
	{/if}

	{#if loading}
		<div class="media-admin__empty">Loading...</div>
	{:else if items.length === 0}
		<div class="media-admin__empty">No media files yet. Upload some!</div>
	{:else}
		<div class="media-admin__body">
			{#if viewMode === 'grid'}
				<div class="media-grid">
					{#each filteredItems as item (item.id)}
						<button
							class="media-grid__item"
							class:media-grid__item--selected={selected?.id === item.id}
							onclick={() => selectItem(item)}
						>
							{#if item.mime_type.startsWith('image/')}
								<img src={item.url} alt={item.alt_text || item.original_filename} loading="lazy" />
							{:else}
								<div class="file-icon">📄</div>
							{/if}
							<span class="media-grid__name">{item.title || item.original_filename}</span>
						</button>
					{/each}
				</div>
			{:else}
				<div class="media-list">
					{#each filteredItems as item (item.id)}
						<button
							class="media-list__item"
							class:media-list__item--selected={selected?.id === item.id}
							onclick={() => selectItem(item)}
						>
							{#if item.mime_type.startsWith('image/')}
								<img class="media-list__thumb" src={item.url} alt="" loading="lazy" />
							{:else}
								<span class="media-list__thumb media-list__thumb--file">📄</span>
							{/if}
							<span class="media-list__name">{item.title || item.original_filename}</span>
							<span class="media-list__meta">{formatSize(item.file_size)}</span>
							<span class="media-list__meta">{item.mime_type}</span>
							<span class="media-list__date">{new Date(item.created_at).toLocaleDateString()}</span>
						</button>
					{/each}
				</div>
			{/if}

			{#if selected}
				<aside class="media-detail">
					<div class="media-detail__preview">
						{#if selected.mime_type.startsWith('image/')}
							<img src={selected.url} alt={selected.alt_text || ''} />
						{:else}
							<div class="file-icon-lg">📄</div>
						{/if}
					</div>
					<p class="media-detail__name">{selected.original_filename}</p>
					<p class="media-detail__meta">{formatSize(selected.file_size)} · {selected.mime_type}</p>
					{#if selected.width && selected.height}
						<p class="media-detail__meta">{selected.width} × {selected.height}px</p>
					{/if}
					<p class="media-detail__meta">
						{new Date(selected.created_at).toLocaleDateString('en-US', {
							month: 'short',
							day: 'numeric',
							year: 'numeric'
						})}
					</p>

					<div class="media-detail__url">
						<input
							id="media-selected-url"
							name="media-url"
							type="text"
							value={selected.url}
							readonly
						/>
						<button onclick={() => copyUrl(selected!.url)}>Copy</button>
					</div>

					<div class="media-detail__fields">
						<label class="media-detail__label" for="media-selected-title">Title</label>
						<input
							id="media-selected-title"
							name="media-title"
							type="text"
							bind:value={editTitle}
							placeholder="Human-readable title…"
						/>

						<label class="media-detail__label" for="media-selected-alt">Alt text</label>
						<input
							id="media-selected-alt"
							name="media-alt"
							type="text"
							bind:value={editAlt}
							placeholder="Describe the image…"
						/>

						<label class="media-detail__label" for="media-selected-caption">Caption</label>
						<input
							id="media-selected-caption"
							name="media-caption"
							type="text"
							bind:value={editCaption}
							placeholder="Optional caption"
						/>
					</div>
					<button onclick={saveMetadata} disabled={savingMeta} class="btn-save-meta">
						{savingMeta ? 'Saving…' : 'Save metadata'}
					</button>

					<div class="media-detail__actions">
						<button class="btn-delete" onclick={() => deleteItem(selected!.id)}>
							Delete permanently
						</button>
					</div>
				</aside>
			{/if}
		</div>

		{#if totalPages > 1}
			<div class="media-admin__pagination">
				<button
					disabled={page <= 1}
					onclick={() => {
						page--;
						loadMedia();
					}}>← Prev</button
				>
				<span>Page {page} of {totalPages}</span>
				<button
					disabled={page >= totalPages}
					onclick={() => {
						page++;
						loadMedia();
					}}>Next →</button
				>
			</div>
		{/if}
	{/if}
</div>

<style>
	.media-admin--drag {
		outline: 2px dashed var(--color-teal, #0fa4af);
		outline-offset: -4px;
	}

	.media-admin__drop-hint {
		text-align: center;
		padding: 2rem;
		font-size: 1.1rem;
		color: var(--color-teal-light, #15c5d1);
		background: rgba(15, 164, 175, 0.06);
		border-radius: 0.5rem;
		margin-bottom: 1rem;
	}

	.media-admin__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.media-admin__header h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #fff;
		margin: 0;
	}

	.media-admin__count {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--color-grey-400, #64748b);
		margin-left: 0.5rem;
	}

	.media-admin__header-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.media-admin__search {
		padding: 0.4rem 0.7rem;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.375rem;
		color: var(--color-grey-200, #e2e8f0);
		font-size: 0.8rem;
		width: 12rem;
	}

	.view-toggle {
		display: flex;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.375rem;
		overflow: hidden;
	}

	.view-toggle button {
		padding: 0.35rem 0.6rem;
		background: transparent;
		border: none;
		color: var(--color-grey-400, #64748b);
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
	}

	.view-toggle button.active {
		background: rgba(15, 164, 175, 0.2);
		color: var(--color-teal-light, #15c5d1);
	}

	/* List view */
	.media-list {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.media-list__item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.4rem 0.75rem;
		border: 1px solid transparent;
		border-radius: 0.375rem;
		background: rgba(255, 255, 255, 0.02);
		cursor: pointer;
		text-align: left;
	}

	.media-list__item:hover {
		border-color: rgba(255, 255, 255, 0.08);
	}
	.media-list__item--selected {
		border-color: var(--color-teal, #0fa4af);
		background: rgba(15, 164, 175, 0.05);
	}

	.media-list__thumb {
		width: 2.5rem;
		height: 2.5rem;
		object-fit: cover;
		border-radius: 0.25rem;
		flex-shrink: 0;
	}

	.media-list__thumb--file {
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(255, 255, 255, 0.04);
		font-size: 1.2rem;
	}

	.media-list__name {
		flex: 1;
		font-size: 0.8rem;
		color: var(--color-grey-200, #e2e8f0);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.media-list__meta,
	.media-list__date {
		font-size: 0.7rem;
		color: var(--color-grey-500, #475569);
		white-space: nowrap;
	}

	/* Detail sidebar fields */
	.media-detail__fields {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		margin-bottom: 0.5rem;
	}

	.media-detail__label {
		font-size: 0.7rem;
		color: var(--color-grey-400, #64748b);
	}

	.media-detail__fields input {
		width: 100%;
		padding: 0.3rem 0.45rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		color: #fff;
		font-size: 0.78rem;
		outline: none;
		box-sizing: border-box;
	}

	.btn-save-meta {
		width: 100%;
		padding: 0.4rem;
		background: var(--color-teal, #0fa4af);
		border: none;
		border-radius: 0.25rem;
		color: #fff;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		margin-bottom: 0.5rem;
	}

	.btn-save-meta:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-upload {
		display: inline-block;
		padding: 0.6rem 1.25rem;
		border-radius: 0.5rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-weight: 600;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.btn-upload:hover {
		opacity: 0.9;
	}

	.media-admin__empty {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400, #64748b);
	}

	.media-admin__body {
		display: flex;
		gap: 1.5rem;
	}

	.media-grid {
		flex: 1;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(8rem, 1fr));
		gap: 0.75rem;
	}

	.media-grid__item {
		border: 2px solid transparent;
		border-radius: 0.375rem;
		overflow: hidden;
		background: rgba(255, 255, 255, 0.03);
		cursor: pointer;
		padding: 0;
		display: flex;
		flex-direction: column;
	}

	.media-grid__item:hover {
		border-color: rgba(255, 255, 255, 0.15);
	}
	.media-grid__item--selected {
		border-color: var(--color-teal, #0fa4af);
	}

	.media-grid__item img {
		width: 100%;
		aspect-ratio: 1;
		object-fit: cover;
	}

	.media-grid__name {
		padding: 0.25rem 0.35rem;
		font-size: 0.65rem;
		color: var(--color-grey-400, #64748b);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-icon,
	.file-icon-lg {
		display: flex;
		align-items: center;
		justify-content: center;
		aspect-ratio: 1;
		font-size: 2rem;
	}

	.file-icon-lg {
		font-size: 4rem;
		padding: 1rem;
	}

	/* Detail sidebar */
	.media-detail {
		width: 18rem;
		flex-shrink: 0;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 0.5rem;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.media-detail__preview img {
		width: 100%;
		height: auto;
		border-radius: 0.375rem;
	}

	.media-detail__name {
		font-size: 0.85rem;
		font-weight: 600;
		color: #fff;
		margin: 0;
		word-break: break-all;
	}

	.media-detail__meta {
		font-size: 0.75rem;
		color: var(--color-grey-400, #64748b);
		margin: 0;
	}

	.media-detail__url {
		display: flex;
		gap: 0.25rem;
	}

	.media-detail__url input {
		flex: 1;
		padding: 0.3rem 0.4rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.2);
		color: var(--color-grey-300, #94a3b8);
		font-size: 0.7rem;
		outline: none;
	}

	.media-detail__url button {
		padding: 0.3rem 0.6rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-teal-light, #15c5d1);
		font-size: 0.7rem;
		cursor: pointer;
	}

	.btn-delete {
		padding: 0.4rem 0.75rem;
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 0.25rem;
		background: transparent;
		color: #ef4444;
		font-size: 0.8rem;
		cursor: pointer;
	}

	.btn-delete:hover {
		background: rgba(239, 68, 68, 0.08);
	}

	.media-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 1.5rem;
	}

	.media-admin__pagination button {
		padding: 0.4rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-grey-300, #94a3b8);
		font-size: 0.8rem;
		cursor: pointer;
	}

	.media-admin__pagination button:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.media-admin__pagination span {
		font-size: 0.8rem;
		color: var(--color-grey-400, #64748b);
	}
</style>
