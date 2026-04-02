<script lang="ts">
	import { api } from '$lib/api/client';
	import type { MediaItem, PaginatedResponse } from '$lib/api/types';

	let items: MediaItem[] = $state([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let uploading = $state(false);
	let selected: MediaItem | null = $state(null);

	$effect(() => {
		loadMedia();
	});

	async function loadMedia() {
		loading = true;
		try {
			const res = await api.get<PaginatedResponse<MediaItem>>(
				`/admin/blog/media?page=${page}&per_page=30`
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
				const media = await api.upload<MediaItem>('/admin/blog/media/upload', formData);
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
		if (!confirm('Delete this media file permanently?')) return;
		try {
			await api.delete(`/admin/blog/media/${id}`);
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

	function copyUrl(url: string) {
		navigator.clipboard.writeText(url);
	}
</script>

<svelte:head>
	<title>Media Library — Admin</title>
</svelte:head>

<div class="media-admin">
	<div class="media-admin__header">
		<h1>Media Library</h1>
		<label class="btn-upload">
			{uploading ? 'Uploading...' : 'Upload Files'}
			<input type="file" accept="image/*,application/pdf" multiple hidden onchange={uploadFiles} />
		</label>
	</div>

	{#if loading}
		<div class="media-admin__empty">Loading...</div>
	{:else if items.length === 0}
		<div class="media-admin__empty">No media files yet. Upload some!</div>
	{:else}
		<div class="media-admin__body">
			<div class="media-grid">
				{#each items as item (item.id)}
					<button
						class="media-grid__item"
						class:media-grid__item--selected={selected?.id === item.id}
						onclick={() => (selected = item)}
					>
						{#if item.mime_type.startsWith('image/')}
							<img src={item.url} alt={item.alt_text || item.original_filename} loading="lazy" />
						{:else}
							<div class="file-icon">📄</div>
						{/if}
						<span class="media-grid__name">{item.original_filename}</span>
					</button>
				{/each}
			</div>

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
						Uploaded {new Date(selected.created_at).toLocaleDateString('en-US', {
							month: 'short',
							day: 'numeric',
							year: 'numeric'
						})}
					</p>

					<div class="media-detail__url">
						<input type="text" value={selected.url} readonly />
						<button onclick={() => copyUrl(selected!.url)}>Copy</button>
					</div>

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
				<button disabled={page <= 1} onclick={() => { page--; loadMedia(); }}>← Prev</button>
				<span>Page {page} of {totalPages}</span>
				<button disabled={page >= totalPages} onclick={() => { page++; loadMedia(); }}>Next →</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	.media-admin__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1.5rem;
	}

	.media-admin__header h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #fff;
		margin: 0;
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

	.btn-upload:hover { opacity: 0.9; }

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

	.media-grid__item:hover { border-color: rgba(255, 255, 255, 0.15); }
	.media-grid__item--selected { border-color: var(--color-teal, #0fa4af); }

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

	.file-icon, .file-icon-lg {
		display: flex;
		align-items: center;
		justify-content: center;
		aspect-ratio: 1;
		font-size: 2rem;
	}

	.file-icon-lg { font-size: 4rem; padding: 1rem; }

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

	.btn-delete:hover { background: rgba(239, 68, 68, 0.08); }

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

	.media-admin__pagination button:disabled { opacity: 0.3; cursor: not-allowed; }

	.media-admin__pagination span {
		font-size: 0.8rem;
		color: var(--color-grey-400, #64748b);
	}
</style>
