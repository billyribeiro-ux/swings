<script lang="ts">
	import { untrack } from 'svelte';
	import { api } from '$lib/api/client';
	import type {
		BlogPostResponse,
		BlogCategory,
		BlogTag,
		BlogRevision,
		MediaItem,
		CreatePostPayload,
		UpdatePostPayload,
		PostStatus
	} from '$lib/api/types';
	import BlogEditor from './BlogEditor.svelte';
	import MediaLibrary from './MediaLibrary.svelte';
	import {
		ArrowLeft,
		CaretRight,
		CaretLeft,
		FloppyDisk,
		PaperPlane,
		Clock,
		Globe,
		Lock,
		Image,
		Trash,
		Plus,
		X,
		ArrowCounterClockwise
	} from 'phosphor-svelte';

	interface Props {
		mode: 'create' | 'edit';
		post?: BlogPostResponse | null;
		onSave: (payload: any) => Promise<BlogPostResponse>;
		onSaved?: (post: BlogPostResponse) => void;
	}

	let { mode, post = null, onSave, onSaved }: Props = $props();

	// Snapshot initial post value — untrack opts out of reactive dependency on the prop
	const p = untrack(() => post);

	// Post fields
	let title = $state(p?.title || '');
	let slug = $state(p?.slug || '');
	let slugManual = $state(false);
	let content = $state(p?.content || '');
	let contentJson: Record<string, unknown> | null = $state(p?.content_json || null);
	let excerpt = $state(p?.excerpt || '');
	let status: PostStatus = $state((p?.status as PostStatus) || 'draft');
	let visibility = $state(p?.visibility || 'public');
	let isSticky = $state(p?.is_sticky || false);
	let allowComments = $state(p?.allow_comments ?? true);
	let metaTitle = $state(p?.meta_title || '');
	let metaDescription = $state(p?.meta_description || '');
	let canonicalUrl = $state(p?.canonical_url || '');
	let ogImageUrl = $state(p?.og_image_url || '');
	let scheduledAt = $state(p?.scheduled_at || '');
	let featuredImageId: string | undefined = $state(undefined);
	let featuredImageUrl = $state(p?.featured_image_url || '');

	// Taxonomy
	let allCategories: BlogCategory[] = $state([]);
	let allTags: BlogTag[] = $state([]);
	let selectedCategoryIds: string[] = $state(p?.categories?.map((c) => c.id) || []);
	let selectedTagIds: string[] = $state(p?.tags?.map((t) => t.id) || []);
	let newCategoryName = $state('');
	let newTagName = $state('');

	// Revisions
	let revisions: BlogRevision[] = $state([]);

	// UI state
	let saving = $state(false);
	let saveMessage = $state('');
	let wordCount = $state(p?.word_count || 0);
	let charCount = $state(0);
	let showMediaLibrary = $state(false);
	let mediaInsertTarget: 'editor' | 'featured' = $state('editor');
	let sidebarOpen = $state(true);
	let autosaveTimer: ReturnType<typeof setTimeout>;

	// Editor reference
	let editorComponent: BlogEditor;

	// Load categories, tags, revisions on mount
	$effect(() => {
		loadTaxonomy();
		if (mode === 'edit' && post) {
			loadRevisions();
		}
	});

	async function loadTaxonomy() {
		try {
			const [cats, tags] = await Promise.all([
				api.get<BlogCategory[]>('/admin/blog/categories'),
				api.get<BlogTag[]>('/admin/blog/tags')
			]);
			allCategories = cats;
			allTags = tags;
		} catch (e) {
			console.error('Failed to load taxonomy', e);
		}
	}

	async function loadRevisions() {
		if (!post) return;
		try {
			revisions = await api.get<BlogRevision[]>(`/admin/blog/posts/${post.id}/revisions`);
		} catch (e) {
			console.error('Failed to load revisions', e);
		}
	}

	function slugify(s: string): string {
		return s
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '');
	}

	function handleTitleInput(e: Event) {
		title = (e.target as HTMLInputElement).value;
		if (!slugManual) {
			slug = slugify(title);
		}
	}

	function handleSlugInput(e: Event) {
		slug = (e.target as HTMLInputElement).value;
		slugManual = true;
	}

	function handleEditorUpdate(html: string, json: Record<string, unknown>) {
		content = html;
		contentJson = json;
		scheduleAutosave();
	}

	function handleWordCount(words: number, chars: number) {
		wordCount = words;
		charCount = chars;
	}

	function scheduleAutosave() {
		if (mode !== 'edit' || !post) return;
		clearTimeout(autosaveTimer);
		autosaveTimer = setTimeout(async () => {
			try {
				await api.post(`/admin/blog/posts/${post!.id}/autosave`, {
					title,
					content,
					content_json: contentJson
				});
			} catch (e) {
				console.error('Autosave failed', e);
			}
		}, 30000);
	}

	function buildPayload(): CreatePostPayload | UpdatePostPayload {
		return {
			title,
			slug,
			content,
			content_json: contentJson || undefined,
			excerpt: excerpt || undefined,
			featured_image_id: featuredImageId || undefined,
			status,
			visibility,
			is_sticky: isSticky,
			allow_comments: allowComments,
			meta_title: metaTitle || undefined,
			meta_description: metaDescription || undefined,
			canonical_url: canonicalUrl || undefined,
			og_image_url: ogImageUrl || undefined,
			category_ids: selectedCategoryIds.length > 0 ? selectedCategoryIds : undefined,
			tag_ids: selectedTagIds.length > 0 ? selectedTagIds : undefined,
			scheduled_at: status === 'scheduled' ? scheduledAt || undefined : undefined
		};
	}

	async function handleSave(overrideStatus?: PostStatus) {
		if (!title.trim()) {
			saveMessage = 'Title is required';
			return;
		}
		saving = true;
		saveMessage = '';
		try {
			const payload = buildPayload();
			if (overrideStatus) payload.status = overrideStatus;
			const result = await onSave(payload);
			saveMessage = overrideStatus === 'published' ? 'Published!' : 'Saved!';
			onSaved?.(result);
			if (mode === 'edit') {
				loadRevisions();
			}
			setTimeout(() => (saveMessage = ''), 3000);
		} catch (e) {
			saveMessage = 'Save failed';
			console.error(e);
		} finally {
			saving = false;
		}
	}

	function openMediaForEditor() {
		mediaInsertTarget = 'editor';
		showMediaLibrary = true;
	}

	function openMediaForFeatured() {
		mediaInsertTarget = 'featured';
		showMediaLibrary = true;
	}

	function handleMediaSelect(media: MediaItem) {
		if (mediaInsertTarget === 'editor') {
			editorComponent?.insertImage(media.url, media.alt_text || '');
		} else {
			featuredImageId = media.id;
			featuredImageUrl = media.url;
		}
	}

	function removeFeaturedImage() {
		featuredImageId = undefined;
		featuredImageUrl = '';
	}

	function toggleCategory(id: string) {
		if (selectedCategoryIds.includes(id)) {
			selectedCategoryIds = selectedCategoryIds.filter((c) => c !== id);
		} else {
			selectedCategoryIds = [...selectedCategoryIds, id];
		}
	}

	function toggleTag(id: string) {
		if (selectedTagIds.includes(id)) {
			selectedTagIds = selectedTagIds.filter((t) => t !== id);
		} else {
			selectedTagIds = [...selectedTagIds, id];
		}
	}

	async function addCategory() {
		if (!newCategoryName.trim()) return;
		try {
			const cat = await api.post<BlogCategory>('/admin/blog/categories', { name: newCategoryName });
			allCategories = [...allCategories, cat];
			selectedCategoryIds = [...selectedCategoryIds, cat.id];
			newCategoryName = '';
		} catch (e) {
			console.error('Failed to create category', e);
		}
	}

	async function addTag() {
		if (!newTagName.trim()) return;
		try {
			const tag = await api.post<BlogTag>('/admin/blog/tags', { name: newTagName });
			allTags = [...allTags, tag];
			selectedTagIds = [...selectedTagIds, tag.id];
			newTagName = '';
		} catch (e) {
			console.error('Failed to create tag', e);
		}
	}

	async function restoreRevision(revId: string) {
		if (
			!post ||
			!confirm('Restore this revision? Current content will be saved as a new revision.')
		)
			return;
		try {
			const result = await api.post<BlogPostResponse>(
				`/admin/blog/posts/${post.id}/revisions/${revId}/restore`
			);
			// Reload
			content = result.content;
			contentJson = result.content_json;
			title = result.title;
			editorComponent?.setContent(result.content);
			loadRevisions();
			saveMessage = 'Revision restored!';
			setTimeout(() => (saveMessage = ''), 3000);
		} catch (e) {
			console.error('Failed to restore revision', e);
		}
	}
</script>

<div class="post-editor">
	<!-- Main content area -->
	<div class="post-editor__main">
		<div class="post-editor__back">
			<a href="/admin/blog">
				<ArrowLeft size={16} weight="bold" />
				<span>Back to posts</span>
			</a>
		</div>

		<!-- Title -->
		<input
			type="text"
			class="post-editor__title"
			value={title}
			oninput={handleTitleInput}
			placeholder="Post title"
		/>

		<!-- Editor -->
		<BlogEditor
			bind:this={editorComponent}
			{content}
			{contentJson}
			onUpdate={handleEditorUpdate}
			onWordCount={handleWordCount}
			onInsertImage={openMediaForEditor}
		/>

		<!-- Status bar -->
		<div class="post-editor__statusbar">
			<span>{wordCount} words · {charCount} characters</span>
			{#if saveMessage}
				<span class="post-editor__save-msg">{saveMessage}</span>
			{/if}
		</div>
	</div>

	<!-- Settings sidebar -->
	<aside class="post-editor__sidebar" class:post-editor__sidebar--collapsed={!sidebarOpen}>
		<button class="sidebar-toggle" onclick={() => (sidebarOpen = !sidebarOpen)}>
			{#if sidebarOpen}
				<CaretRight size={16} weight="bold" />
			{:else}
				<CaretLeft size={16} weight="bold" />
			{/if}
			<span>Settings</span>
		</button>

		{#if sidebarOpen}
			<!-- Publish actions -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Publish</h3>
				<div class="sidebar-section__content">
					<div class="publish-actions">
						<button
							class="publish-btn publish-btn--draft"
							onclick={() => handleSave('draft')}
							disabled={saving}
						>
							<FloppyDisk size={16} weight="bold" />
							<span>Save Draft</span>
						</button>
						<button
							class="publish-btn publish-btn--publish"
							onclick={() => handleSave('published')}
							disabled={saving}
						>
							<PaperPlane size={16} weight="bold" />
							<span>{mode === 'edit' && post?.status === 'published' ? 'Update' : 'Publish'}</span>
						</button>
					</div>
				</div>
			</div>

			<!-- Status & Visibility -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Status & Visibility</h3>
				<div class="sidebar-section__content">
					<label class="sidebar-field">
						<span class="sidebar-field__label">Status</span>
						<select class="sidebar-field__select" bind:value={status}>
							<option value="draft">Draft</option>
							<option value="pending_review">Pending Review</option>
							<option value="published">Published</option>
							<option value="private">Private</option>
							<option value="scheduled">Scheduled</option>
						</select>
					</label>

					<label class="sidebar-field">
						<span class="sidebar-field__label">Visibility</span>
						<select class="sidebar-field__select" bind:value={visibility}>
							<option value="public">Public</option>
							<option value="private">Private</option>
							<option value="password">Password Protected</option>
						</select>
					</label>

					{#if status === 'scheduled'}
						<label class="sidebar-field">
							<span class="sidebar-field__label">Schedule for</span>
							<input type="datetime-local" class="sidebar-field__input" bind:value={scheduledAt} />
						</label>
					{/if}

					<label class="sidebar-field sidebar-field--inline">
						<input type="checkbox" bind:checked={isSticky} />
						<span>Sticky post</span>
					</label>

					<label class="sidebar-field sidebar-field--inline">
						<input type="checkbox" bind:checked={allowComments} />
						<span>Allow comments</span>
					</label>
				</div>
			</div>

			<!-- Permalink -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Permalink</h3>
				<div class="sidebar-section__content">
					<label class="sidebar-field">
						<span class="sidebar-field__label">Slug</span>
						<input
							type="text"
							class="sidebar-field__input"
							value={slug}
							oninput={handleSlugInput}
							placeholder="post-slug"
						/>
					</label>
					<div class="slug-preview">
						/blog/{slug || '...'}
					</div>
				</div>
			</div>

			<!-- Categories -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Categories</h3>
				<div class="sidebar-section__content">
					<div class="category-list">
						{#each allCategories as cat (cat.id)}
							<label class="category-item">
								<input
									type="checkbox"
									checked={selectedCategoryIds.includes(cat.id)}
									onchange={() => toggleCategory(cat.id)}
								/>
								<span>{cat.name}</span>
							</label>
						{/each}
					</div>
					<div class="add-taxonomy">
						<input
							type="text"
							class="add-taxonomy__input"
							bind:value={newCategoryName}
							placeholder="New category"
							onkeydown={(e) => e.key === 'Enter' && addCategory()}
						/>
						<button class="add-taxonomy__btn" onclick={addCategory}>
							<Plus size={14} weight="bold" />
						</button>
					</div>
				</div>
			</div>

			<!-- Tags -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Tags</h3>
				<div class="sidebar-section__content">
					<div class="tag-list">
						{#each allTags as tag (tag.id)}
							<button
								class="tag-pill"
								class:tag-pill--selected={selectedTagIds.includes(tag.id)}
								onclick={() => toggleTag(tag.id)}
							>
								<span>{tag.name}</span>
								{#if selectedTagIds.includes(tag.id)}
									<X size={12} weight="bold" />
								{/if}
							</button>
						{/each}
					</div>
					<div class="add-taxonomy">
						<input
							type="text"
							class="add-taxonomy__input"
							bind:value={newTagName}
							placeholder="New tag"
							onkeydown={(e) => e.key === 'Enter' && addTag()}
						/>
						<button class="add-taxonomy__btn" onclick={addTag}>
							<Plus size={14} weight="bold" />
						</button>
					</div>
				</div>
			</div>

			<!-- Featured Image -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Featured Image</h3>
				<div class="sidebar-section__content">
					{#if featuredImageUrl}
						<div class="featured-preview">
							<img src={featuredImageUrl} alt="Featured" />
							<button class="featured-remove" onclick={removeFeaturedImage}>
								<Trash size={14} weight="bold" />
								<span>Remove</span>
							</button>
						</div>
					{:else}
						<button class="featured-set" onclick={openMediaForFeatured}>
							<Image size={18} weight="bold" />
							<span>Set featured image</span>
						</button>
					{/if}
				</div>
			</div>

			<!-- Excerpt -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Excerpt</h3>
				<div class="sidebar-section__content">
					<textarea
						class="sidebar-field__textarea"
						bind:value={excerpt}
						placeholder="Write a short excerpt..."
						rows="3"
					></textarea>
				</div>
			</div>

			<!-- SEO -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">SEO Settings</h3>
				<div class="sidebar-section__content">
					<label class="sidebar-field">
						<span class="sidebar-field__label">
							Meta title <span class="char-count">({metaTitle.length}/60)</span>
						</span>
						<input
							type="text"
							class="sidebar-field__input"
							bind:value={metaTitle}
							placeholder="SEO title"
							maxlength="60"
						/>
					</label>

					<label class="sidebar-field">
						<span class="sidebar-field__label">
							Meta description <span class="char-count">({metaDescription.length}/160)</span>
						</span>
						<textarea
							class="sidebar-field__textarea"
							bind:value={metaDescription}
							placeholder="SEO description"
							maxlength="160"
							rows="3"
						></textarea>
					</label>

					<label class="sidebar-field">
						<span class="sidebar-field__label">Canonical URL</span>
						<input
							type="url"
							class="sidebar-field__input"
							bind:value={canonicalUrl}
							placeholder="https://..."
						/>
					</label>

					<label class="sidebar-field">
						<span class="sidebar-field__label">OG Image URL</span>
						<input
							type="url"
							class="sidebar-field__input"
							bind:value={ogImageUrl}
							placeholder="https://..."
						/>
					</label>
				</div>
			</div>

			<!-- Revisions -->
			{#if mode === 'edit' && revisions.length > 0}
				<div class="sidebar-section">
					<h3 class="sidebar-section__title">Revisions ({revisions.length})</h3>
					<div class="sidebar-section__content">
						<div class="revision-list">
							{#each revisions.slice(0, 10) as rev (rev.id)}
								<div class="revision-item">
									<div class="revision-item__info">
										<span class="revision-item__number">#{rev.revision_number}</span>
										<span class="revision-item__author">{rev.author_name}</span>
										<span class="revision-item__date">
											{new Date(rev.created_at).toLocaleDateString('en-US', {
												month: 'short',
												day: 'numeric',
												hour: '2-digit',
												minute: '2-digit'
											})}
										</span>
									</div>
									<button class="revision-item__restore" onclick={() => restoreRevision(rev.id)}>
										<ArrowCounterClockwise size={14} weight="bold" />
										<span>Restore</span>
									</button>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/if}
		{/if}
	</aside>
</div>

<MediaLibrary
	open={showMediaLibrary}
	onClose={() => (showMediaLibrary = false)}
	onSelect={handleMediaSelect}
/>

<style>
	.post-editor {
		display: flex;
		gap: 1.5rem;
		min-height: calc(100vh - 8rem);
	}

	.post-editor__main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.post-editor__back a {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-grey-400, #64748b);
		text-decoration: none;
		font-size: 0.8rem;
	}

	.post-editor__back a:hover {
		color: var(--color-teal-light, #15c5d1);
	}

	.post-editor__title {
		width: 100%;
		padding: 0.75rem 1rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.5rem;
		background: rgba(255, 255, 255, 0.03);
		color: #fff;
		font-size: 1.5rem;
		font-weight: 700;
		outline: none;
	}

	.post-editor__title:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.post-editor__title::placeholder {
		color: rgba(255, 255, 255, 0.25);
	}

	.post-editor__statusbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 0;
		font-size: 0.75rem;
		color: var(--color-grey-400, #64748b);
	}

	.post-editor__save-msg {
		color: var(--color-teal-light, #15c5d1);
		font-weight: 600;
	}

	/* Sidebar */
	.post-editor__sidebar {
		width: 20rem;
		flex-shrink: 0;
		overflow-y: auto;
		max-height: calc(100vh - 8rem);
	}

	.post-editor__sidebar--collapsed {
		width: auto;
	}

	.sidebar-toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 0.375rem;
		background: rgba(255, 255, 255, 0.03);
		color: var(--color-grey-300, #94a3b8);
		font-size: 0.8rem;
		cursor: pointer;
		text-align: left;
		margin-bottom: 0.75rem;
	}

	.sidebar-toggle:hover {
		background: rgba(255, 255, 255, 0.06);
	}

	.sidebar-section {
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 0.5rem;
		margin-bottom: 0.75rem;
		overflow: hidden;
	}

	.sidebar-section__title {
		padding: 0.6rem 0.75rem;
		margin: 0;
		font-size: 0.8rem;
		font-weight: 700;
		color: var(--color-grey-200, #e2e8f0);
		background: rgba(255, 255, 255, 0.03);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.sidebar-section__content {
		padding: 0.75rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	/* Publish actions */
	.publish-actions {
		display: flex;
		gap: 0.5rem;
	}

	.publish-btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.35rem;
		padding: 0.5rem;
		border: none;
		border-radius: 0.375rem;
		font-size: 0.8rem;
		font-weight: 700;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.publish-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.publish-btn--draft {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-grey-200, #e2e8f0);
	}

	.publish-btn--draft:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.12);
	}

	.publish-btn--publish {
		background: var(--color-teal, #0fa4af);
		color: #fff;
	}

	.publish-btn--publish:hover:not(:disabled) {
		opacity: 0.9;
	}

	/* Fields */
	.sidebar-field {
		display: block;
	}

	.sidebar-field--inline {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8rem;
		color: var(--color-grey-300, #94a3b8);
	}

	.sidebar-field__label {
		display: block;
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--color-grey-400, #64748b);
		margin-bottom: 0.25rem;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.char-count {
		font-weight: 400;
		text-transform: none;
		letter-spacing: 0;
	}

	.sidebar-field__input,
	.sidebar-field__select,
	.sidebar-field__textarea {
		width: 100%;
		padding: 0.35rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.2);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
	}

	.sidebar-field__textarea {
		resize: vertical;
		font-family: inherit;
	}

	.sidebar-field__input:focus,
	.sidebar-field__select:focus,
	.sidebar-field__textarea:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.slug-preview {
		font-size: 0.7rem;
		color: var(--color-grey-400, #64748b);
		word-break: break-all;
	}

	/* Categories */
	.category-list {
		max-height: 10rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.category-item {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
		color: var(--color-grey-300, #94a3b8);
		cursor: pointer;
	}

	.category-item:hover {
		color: #fff;
	}

	/* Tags */
	.tag-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}

	.tag-pill {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.2rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 1rem;
		background: transparent;
		color: var(--color-grey-300, #94a3b8);
		font-size: 0.7rem;
		cursor: pointer;
	}

	.tag-pill:hover {
		border-color: rgba(255, 255, 255, 0.2);
	}

	.tag-pill--selected {
		background: rgba(15, 164, 175, 0.15);
		border-color: var(--color-teal, #0fa4af);
		color: var(--color-teal-light, #15c5d1);
	}

	/* Add taxonomy inline */
	.add-taxonomy {
		display: flex;
		gap: 0.25rem;
	}

	.add-taxonomy__input {
		flex: 1;
		padding: 0.3rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.2);
		color: #fff;
		font-size: 0.75rem;
		outline: none;
	}

	.add-taxonomy__input:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.add-taxonomy__btn {
		padding: 0.3rem 0.6rem;
		border: none;
		border-radius: 0.25rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-size: 0.8rem;
		font-weight: 700;
		cursor: pointer;
	}

	/* Featured image */
	.featured-preview {
		position: relative;
	}

	.featured-preview img {
		width: 100%;
		height: auto;
		border-radius: 0.375rem;
	}

	.featured-remove {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.2rem 0.5rem;
		border: none;
		border-radius: 0.25rem;
		background: rgba(239, 68, 68, 0.8);
		color: #fff;
		font-size: 0.7rem;
		cursor: pointer;
	}

	.featured-set {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.75rem;
		border: 2px dashed rgba(255, 255, 255, 0.1);
		border-radius: 0.375rem;
		background: transparent;
		color: var(--color-grey-400, #64748b);
		font-size: 0.8rem;
		cursor: pointer;
	}

	.featured-set:hover {
		border-color: var(--color-teal, #0fa4af);
		color: var(--color-teal-light, #15c5d1);
	}

	/* Revisions */
	.revision-list {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		max-height: 12rem;
		overflow-y: auto;
	}

	.revision-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.35rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		font-size: 0.75rem;
	}

	.revision-item__info {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.revision-item__number {
		font-weight: 700;
		color: var(--color-grey-200, #e2e8f0);
	}

	.revision-item__author {
		color: var(--color-grey-400, #64748b);
	}

	.revision-item__date {
		color: var(--color-grey-500, #475569);
		font-size: 0.7rem;
	}

	.revision-item__restore {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.2rem 0.5rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-teal-light, #15c5d1);
		font-size: 0.7rem;
		cursor: pointer;
	}

	.revision-item__restore:hover {
		background: rgba(15, 164, 175, 0.1);
	}

	@media (max-width: 768px) {
		.post-editor {
			flex-direction: column;
		}

		.post-editor__sidebar {
			width: 100%;
			max-height: none;
		}
	}
</style>
