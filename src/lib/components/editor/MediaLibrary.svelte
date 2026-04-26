<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '$lib/api/client';
	import type { MediaItem, PaginatedResponse } from '$lib/api/types';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

	interface Props {
		open: boolean;
		onClose: () => void;
		onSelect: (media: MediaItem) => void;
	}

	let { open, onClose, onSelect }: Props = $props();

	let items: MediaItem[] = $state([]);
	let _total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(false);
	let uploading = $state(false);
	let dragOver = $state(false);
	let selected: MediaItem | null = $state(null);
	let editTitle = $state('');
	let editAlt = $state('');
	let editCaption = $state('');

	// `loadMedia()` mutates several `$state` cells (items, _total, totalPages,
	// loading) that are read elsewhere. Without `untrack`, the $effect would
	// re-fire reactively on each mutation and crash with
	// `effect_update_depth_exceeded`. We only want the load to happen when
	// `open` flips to true.
	$effect(() => {
		if (open) {
			untrack(loadMedia);
		}
	});

	async function loadMedia() {
		loading = true;
		try {
			const res = await api.get<PaginatedResponse<MediaItem>>(
				`/api/admin/blog/media?page=${page}&per_page=20`
			);
			items = res.data;
			_total = res.total;
			totalPages = res.total_pages;
		} catch (e) {
			console.error('Failed to load media', e);
		} finally {
			loading = false;
		}
	}

	async function uploadFile(file: File, providedTitle?: string) {
		uploading = true;
		try {
			const formData = new FormData();
			formData.append('file', file);
			if (providedTitle?.trim()) formData.append('title', providedTitle.trim());
			const media = await api.upload<MediaItem>('/api/admin/blog/media/upload', formData);
			items = [media, ...items];
			_total += 1;
			selected = media;
			editTitle = media.title || '';
			editAlt = media.alt_text || '';
			editCaption = media.caption || '';
		} catch (e) {
			console.error('Upload failed', e);
		} finally {
			uploading = false;
		}
	}

	function handleFileInput(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (file) uploadFile(file);
		input.value = '';
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		const file = e.dataTransfer?.files?.[0];
		if (file) uploadFile(file);
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		dragOver = true;
	}

	function handleDragLeave() {
		dragOver = false;
	}

	function selectItem(item: MediaItem) {
		selected = item;
		editTitle = item.title || '';
		editAlt = item.alt_text || '';
		editCaption = item.caption || '';
	}

	async function saveAndInsert() {
		if (!selected) return;
		// Save title/alt/caption if changed
		if (
			editTitle !== (selected.title || '') ||
			editAlt !== (selected.alt_text || '') ||
			editCaption !== (selected.caption || '')
		) {
			try {
				await api.put(`/api/admin/blog/media/${selected.id}`, {
					title: editTitle || undefined,
					alt_text: editAlt || undefined,
					caption: editCaption || undefined
				});
				selected = {
					...selected,
					title: editTitle || null,
					alt_text: editAlt || null,
					caption: editCaption || null
				};
			} catch (e) {
				console.error('Failed to update media', e);
			}
		}
		onSelect(selected);
		onClose();
	}

	async function deleteSelected() {
		if (!selected) return;
		const ok = await confirmDialog({
			title: 'Delete this media file?',
			message: 'The file will be permanently removed and any post that embeds it will lose the asset.',
			confirmLabel: 'Delete',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.delete(`/api/admin/blog/media/${selected.id}`);
			items = items.filter((i) => i.id !== selected!.id);
			_total -= 1;
			selected = null;
		} catch (e) {
			console.error('Failed to delete media', e);
		}
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	const FOCUSABLE =
		'button:not([disabled]), input:not([disabled]), [href], select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

	function focusTrap(node: HTMLElement) {
		const firstFocusable = node.querySelector<HTMLElement>(FOCUSABLE);
		firstFocusable?.focus();

		function onKeydown(e: KeyboardEvent) {
			if (e.key !== 'Tab') return;
			const focusable = [...node.querySelectorAll<HTMLElement>(FOCUSABLE)];
			if (!focusable.length) return;
			const first = focusable[0];
			const last = focusable[focusable.length - 1];
			if (e.shiftKey && document.activeElement === first) {
				e.preventDefault();
				last.focus();
			} else if (!e.shiftKey && document.activeElement === last) {
				e.preventDefault();
				first.focus();
			}
		}

		node.addEventListener('keydown', onKeydown);
		return () => node.removeEventListener('keydown', onKeydown);
	}

	function escapeClose(node: HTMLElement) {
		function onKeydown(e: KeyboardEvent) {
			if (e.key === 'Escape') onClose();
		}
		node.addEventListener('keydown', onKeydown);
		return () => node.removeEventListener('keydown', onKeydown);
	}
</script>

{#if open}
	<div class="media-overlay" role="dialog" aria-modal="true" aria-label="Media Library">
		<div class="media-modal" {@attach focusTrap} {@attach escapeClose}>
			<div class="media-modal__header">
				<h2 class="media-modal__title">Media Library</h2>
				<button class="media-modal__close" onclick={onClose}>✕</button>
			</div>

			<div class="media-modal__body">
				<!-- Left: grid + upload -->
				<div class="media-modal__grid-area">
					<!-- Upload zone -->
					<div
						class="media-upload"
						class:media-upload--drag={dragOver}
						ondrop={handleDrop}
						ondragover={handleDragOver}
						ondragleave={handleDragLeave}
						role="button"
						tabindex="0"
					>
						{#if uploading}
							<p class="media-upload__text">Uploading...</p>
						{:else}
							<p class="media-upload__text">
								Drag & drop files here, or
								<label class="media-upload__label">
									browse
									<input
										id="media-lib-file"
										name="file"
										type="file"
										accept="image/*,application/pdf"
										hidden
										onchange={handleFileInput}
									/>
								</label>
							</p>
						{/if}
					</div>

					<!-- Grid -->
					{#if loading}
						<div class="media-loading">Loading media...</div>
					{:else if items.length === 0}
						<div class="media-empty">No media uploaded yet.</div>
					{:else}
						<div class="media-grid">
							{#each items as item (item.id)}
								<button
									class="media-grid__item"
									class:media-grid__item--selected={selected?.id === item.id}
									onclick={() => selectItem(item)}
								>
									{#if item.mime_type.startsWith('image/')}
										<img
											src={item.url}
											alt={item.alt_text || item.original_filename}
											loading="lazy"
										/>
									{:else}
										<div class="media-grid__file-icon">📄</div>
									{/if}
								</button>
							{/each}
						</div>

						<!-- Pagination -->
						{#if totalPages > 1}
							<div class="media-pagination">
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

				<!-- Right: details sidebar -->
				{#if selected}
					<div class="media-details">
						<div class="media-details__preview">
							{#if selected.mime_type.startsWith('image/')}
								<img src={selected.url} alt={selected.alt_text || ''} />
							{:else}
								<div class="media-details__file-icon">📄</div>
							{/if}
						</div>

						<div class="media-details__info">
							<p class="media-details__filename">{selected.original_filename}</p>
							<p class="media-details__meta">
								{formatSize(selected.file_size)} · {selected.mime_type}
							</p>
							{#if selected.width && selected.height}
								<p class="media-details__meta">{selected.width} × {selected.height}px</p>
							{/if}
						</div>

						<label class="media-details__label">
							Title
							<input
								id="media-lib-title"
								name="media-title"
								type="text"
								class="media-details__input"
								bind:value={editTitle}
								placeholder="Human-readable title..."
							/>
						</label>

						<label class="media-details__label">
							Alt text
							<input
								id="media-lib-alt"
								name="media-alt-text"
								type="text"
								class="media-details__input"
								bind:value={editAlt}
								placeholder="Describe this image..."
							/>
						</label>

						<label class="media-details__label">
							Caption
							<input
								id="media-lib-caption"
								name="media-caption"
								type="text"
								class="media-details__input"
								bind:value={editCaption}
								placeholder="Optional caption"
							/>
						</label>

						<div class="media-details__url">
							<label class="media-details__label">
								URL
								<input
									id="media-lib-url"
									name="media-url"
									type="text"
									class="media-details__input"
									value={selected.url}
									readonly
								/>
							</label>
						</div>

						<div class="media-details__actions">
							<button class="media-details__btn media-details__btn--insert" onclick={saveAndInsert}>
								Insert into post
							</button>
							<button
								class="media-details__btn media-details__btn--delete"
								onclick={deleteSelected}
							>
								Delete
							</button>
						</div>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.media-overlay {
		position: fixed;
		inset: 0;
		z-index: 10000;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2rem;
	}

	.media-modal {
		width: 100%;
		max-width: 60rem;
		max-height: 85vh;
		background: var(--color-navy-deep, #0d1321);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.75rem;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.media-modal__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.media-modal__title {
		font-size: 1.125rem;
		font-weight: 700;
		color: #fff;
		margin: 0;
	}

	.media-modal__close {
		background: none;
		border: none;
		color: var(--color-grey-400, #64748b);
		font-size: 1.25rem;
		cursor: pointer;
		padding: 0.25rem;
	}

	.media-modal__close:hover {
		color: #fff;
	}

	.media-modal__body {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.media-modal__grid-area {
		flex: 1;
		overflow-y: auto;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	/* Upload zone */
	.media-upload {
		border: 2px dashed rgba(255, 255, 255, 0.12);
		border-radius: 0.5rem;
		padding: 1.5rem;
		text-align: center;
		transition:
			border-color 0.2s,
			background 0.2s;
	}

	.media-upload--drag {
		border-color: var(--color-teal, #0fa4af);
		background: rgba(15, 164, 175, 0.05);
	}

	.media-upload__text {
		color: var(--color-grey-400, #64748b);
		font-size: 0.875rem;
		margin: 0;
	}

	.media-upload__label {
		color: var(--color-teal-light, #15c5d1);
		cursor: pointer;
		text-decoration: underline;
	}

	/* Grid */
	.media-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(7rem, 1fr));
		gap: 0.5rem;
	}

	.media-grid__item {
		aspect-ratio: 1;
		border: 2px solid transparent;
		border-radius: 0.375rem;
		overflow: hidden;
		background: rgba(255, 255, 255, 0.04);
		cursor: pointer;
		padding: 0;
	}

	.media-grid__item:hover {
		border-color: rgba(255, 255, 255, 0.2);
	}

	.media-grid__item--selected {
		border-color: var(--color-teal, #0fa4af);
	}

	.media-grid__item img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.media-grid__file-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		font-size: 2rem;
	}

	.media-loading,
	.media-empty {
		text-align: center;
		color: var(--color-grey-400, #64748b);
		padding: 2rem;
		font-size: 0.875rem;
	}

	.media-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		padding: 0.5rem;
	}

	.media-pagination button {
		padding: 0.35rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-grey-300, #94a3b8);
		font-size: 0.8rem;
		cursor: pointer;
	}

	.media-pagination button:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.media-pagination span {
		font-size: 0.8rem;
		color: var(--color-grey-400, #64748b);
	}

	/* Details sidebar */
	.media-details {
		width: 16rem;
		border-left: 1px solid rgba(255, 255, 255, 0.08);
		padding: 1rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.media-details__preview {
		border-radius: 0.375rem;
		overflow: hidden;
		background: rgba(255, 255, 255, 0.04);
	}

	.media-details__preview img {
		width: 100%;
		height: auto;
		display: block;
	}

	.media-details__file-icon {
		padding: 2rem;
		text-align: center;
		font-size: 3rem;
	}

	.media-details__info {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.media-details__filename {
		font-size: 0.8rem;
		color: #fff;
		font-weight: 600;
		margin: 0;
		word-break: break-all;
	}

	.media-details__meta {
		font-size: 0.75rem;
		color: var(--color-grey-400, #64748b);
		margin: 0;
	}

	.media-details__label {
		display: block;
		font-size: 0.75rem;
		color: var(--color-grey-400, #64748b);
	}

	.media-details__input {
		display: block;
		width: 100%;
		padding: 0.35rem 0.5rem;
		margin-top: 0.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.3);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
	}

	.media-details__input:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.media-details__url {
		word-break: break-all;
	}

	.media-details__actions {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-top: auto;
	}

	.media-details__btn {
		padding: 0.5rem;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		text-align: center;
	}

	.media-details__btn--insert {
		background: var(--color-teal, #0fa4af);
		color: #fff;
	}

	.media-details__btn--insert:hover {
		opacity: 0.9;
	}

	.media-details__btn--delete {
		background: transparent;
		border: 1px solid rgba(239, 68, 68, 0.4);
		color: #ef4444;
	}

	.media-details__btn--delete:hover {
		background: rgba(239, 68, 68, 0.1);
	}
</style>
