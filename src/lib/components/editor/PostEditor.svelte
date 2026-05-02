<script lang="ts">
	import { untrack } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import { toast } from '$lib/stores/toast.svelte';
	import type {
		BlogPostResponse,
		BlogCategory,
		BlogTag,
		BlogRevision,
		MediaItem,
		CreatePostPayload,
		UpdatePostPayload,
		PostStatus,
		UserResponse,
		PaginatedResponse,
		PostMeta
	} from '$lib/api/types';
	import BlogEditor from './BlogEditor.svelte';
	import MediaLibrary from './MediaLibrary.svelte';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import {
		buildPostPayload,
		formatAutosaveRelative,
		slugifyPostTitle
	} from './post-editor-utils';
	import {
		ArrowLeft,
		CaretRight,
		CaretLeft,
		FloppyDisk,
		PaperPlane,
		Image,
		Trash,
		Plus,
		X,
		ArrowCounterClockwise
	} from 'phosphor-svelte';

	interface Props {
		mode: 'create' | 'edit';
		post?: BlogPostResponse | null;
		onSave: (payload: CreatePostPayload | UpdatePostPayload) => Promise<BlogPostResponse>;
		onSaved?: (post: BlogPostResponse) => void;
		/** Called after a successful restore so the parent can reload the post. */
		onRestored?: () => void | Promise<void>;
	}

	let { mode, post = null, onSave, onSaved, onRestored }: Props = $props();

	// Snapshot initial post value via `untrack` so this read is not registered
	// as a reactive dependency — `$state` initialisers below should reflect the
	// initial post and not the prop's later identity.
	const p = untrack(() => post);

	// Post fields
	let title = $state(p?.title || '');
	let slug = $state(p?.slug || '');
	let slugManual = $state(false);
	let content = $state(p?.content || '');
	// Opaque ProseMirror JSON tree — no nested fields are reactive consumers, so
	// `$state.raw` skips the deep proxy and avoids per-keystroke proxy overhead.
	let contentJson: Record<string, unknown> | null = $state.raw(p?.content_json || null);
	let excerpt = $state(p?.excerpt || '');
	let status: PostStatus = $state((p?.status as PostStatus) || 'draft');
	let visibility = $state(p?.visibility || 'public');
	let postPassword = $state('');
	let postFormat = $state(p?.format || 'standard');
	let isSticky = $state(p?.is_sticky || false);
	let allowComments = $state(p?.allow_comments ?? true);
	let metaTitle = $state(p?.meta_title || '');
	let metaDescription = $state(p?.meta_description || '');
	let canonicalUrl = $state(p?.canonical_url || '');
	let ogImageUrl = $state(p?.og_image_url || '');
	let scheduledAt = $state(p?.scheduled_at || '');
	let featuredImageId: string | undefined = $state(undefined);
	let featuredImageUrl = $state(p?.featured_image_url || '');
	let focalX = $state(0.5);
	let focalY = $state(0.5);

	// Taxonomy
	let allCategories: BlogCategory[] = $state([]);
	let allTags: BlogTag[] = $state([]);
	let allAdmins: UserResponse[] = $state([]);
	let authorId = $state(p?.author_id || '');
	const selectedCategoryIds = new SvelteSet<string>(p?.categories?.map((c) => c.id));
	const selectedTagIds = new SvelteSet<string>(p?.tags?.map((t) => t.id));
	let newCategoryName = $state('');
	let newTagName = $state('');

	// Revisions
	let revisions: BlogRevision[] = $state([]);

	// Custom fields (post meta)
	let postMeta: PostMeta[] = $state(p?.meta || []);
	let newMetaKey = $state('');
	let newMetaValue = $state('');

	// UI state
	let saving = $state(false);
	let saveMessage = $state('');
	let autosaveStatus: 'idle' | 'pending' | 'saving' | 'saved' | 'error' = $state('idle');
	let lastSavedAt: Date | null = $state(null);
	let wordCount = $state(p?.word_count || 0);

	// SEO analysis (derived — must be after wordCount)
	const seoChecks = $derived([
		{
			label: 'Title length (50–60 chars)',
			pass: metaTitle.length >= 40 && metaTitle.length <= 60,
			warn: metaTitle.length > 0 && (metaTitle.length < 40 || metaTitle.length > 60),
			value: `${metaTitle.length} chars`
		},
		{
			label: 'Meta description (120–160 chars)',
			pass: metaDescription.length >= 120 && metaDescription.length <= 160,
			warn:
				metaDescription.length > 0 &&
				(metaDescription.length < 120 || metaDescription.length > 160),
			value: `${metaDescription.length} chars`
		},
		{
			label: 'Word count (300+)',
			pass: wordCount >= 300,
			warn: wordCount >= 150 && wordCount < 300,
			value: `${wordCount} words`
		},
		{ label: 'Has featured image', pass: !!featuredImageUrl, warn: false, value: '' },
		{ label: 'Has excerpt', pass: excerpt.length > 0, warn: false, value: '' },
		{
			label: 'Has focus keyword in title',
			pass: metaTitle.toLowerCase().includes(title.split(' ')[0]?.toLowerCase() ?? ''),
			warn: false,
			value: ''
		}
	]);
	let charCount = $state(0);
	let showMediaLibrary = $state(false);
	let mediaInsertTarget: 'editor' | 'featured' = $state('editor');
	let sidebarOpen = $state(true);
	let autosaveTimer: ReturnType<typeof setTimeout> | undefined;
	let trashing = $state(false);
	let restoring = $state(false);
	let deletingPermanent = $state(false);

	// Editor reference
	let editorComponent: BlogEditor | undefined = $state();

	// Initial fetch of taxonomy + (in edit mode) revisions/meta. Wrapped in
	// `untrack` so subsequent prop mutations to `post` don't re-fire these
	// network calls — the load is mount-only by design.
	$effect(() => {
		untrack(() => {
			loadTaxonomy();
			if (mode === 'edit' && post) {
				loadRevisions();
				loadMeta();
			}
		});
	});

	// Cancel pending autosave on unmount so a stale callback doesn't fire
	// against a destroyed component.
	$effect(() => () => {
		if (autosaveTimer) clearTimeout(autosaveTimer);
	});

	async function loadMeta() {
		if (!post) return;
		try {
			postMeta = await api.get<PostMeta[]>(`/api/admin/blog/posts/${post.id}/meta`);
		} catch (e) {
			console.error('Failed to load post meta', e);
		}
	}

	async function addMeta() {
		if (!newMetaKey.trim() || !post) return;
		try {
			const item = await api.post<PostMeta>(`/api/admin/blog/posts/${post.id}/meta`, {
				meta_key: newMetaKey.trim(),
				meta_value: newMetaValue
			});
			const idx = postMeta.findIndex((m) => m.meta_key === item.meta_key);
			if (idx >= 0) postMeta[idx] = item;
			else postMeta = [...postMeta, item];
			newMetaKey = '';
			newMetaValue = '';
		} catch (e) {
			console.error('Failed to save meta', e);
		}
	}

	async function deleteMeta(key: string) {
		if (!post) return;
		try {
			await api.delete(`/api/admin/blog/posts/${post.id}/meta/${encodeURIComponent(key)}`);
			postMeta = postMeta.filter((m) => m.meta_key !== key);
		} catch (e) {
			console.error('Failed to delete meta', e);
		}
	}

	async function loadTaxonomy() {
		try {
			const [cats, tags, members] = await Promise.all([
				api.get<BlogCategory[]>('/api/admin/blog/categories'),
				api.get<BlogTag[]>('/api/admin/blog/tags'),
				api.get<PaginatedResponse<UserResponse>>('/api/admin/members?per_page=100')
			]);
			allCategories = cats;
			allTags = tags;
			allAdmins = members.data;
		} catch (e) {
			console.error('Failed to load taxonomy', e);
		}
	}

	async function loadRevisions() {
		if (!post) return;
		try {
			revisions = await api.get<BlogRevision[]>(`/api/admin/blog/posts/${post.id}/revisions`);
		} catch (e) {
			console.error('Failed to load revisions', e);
		}
	}

	function handleTitleInput(e: Event) {
		title = (e.target as HTMLInputElement).value;
		if (!slugManual) {
			slug = slugifyPostTitle(title);
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
		if (mode !== 'edit' || !post || post.status === 'trash') return;
		autosaveStatus = 'pending';
		clearTimeout(autosaveTimer);
		autosaveTimer = setTimeout(async () => {
			autosaveStatus = 'saving';
			try {
				await api.post(`/api/admin/blog/posts/${post!.id}/autosave`, {
					title,
					content,
					content_json: contentJson
				});
				autosaveStatus = 'saved';
				lastSavedAt = new Date();
				setTimeout(() => {
					if (autosaveStatus === 'saved') autosaveStatus = 'idle';
				}, 30000);
			} catch (e) {
				console.error('Autosave failed', e);
				autosaveStatus = 'error';
			}
		}, 30000);
	}

	function buildPayload(): CreatePostPayload | UpdatePostPayload {
		return buildPostPayload({
			title,
			slug,
			content,
			contentJson,
			excerpt,
			featuredImageId,
			status,
			visibility,
			isSticky,
			allowComments,
			metaTitle,
			metaDescription,
			canonicalUrl,
			ogImageUrl,
			selectedCategoryIds: [...selectedCategoryIds],
			selectedTagIds: [...selectedTagIds],
			scheduledAt,
			postPassword,
			authorId,
			postFormat
		});
	}

	async function handleSave(overrideStatus?: PostStatus) {
		if (!title.trim()) {
			saveMessage = 'Title is required';
			toast.error('Title is required');
			return;
		}
		// Cancel any pending autosave so it can't race the explicit save and
		// stomp the new server-authoritative state on its next callback.
		if (autosaveTimer) {
			clearTimeout(autosaveTimer);
			autosaveStatus = 'idle';
		}
		saving = true;
		saveMessage = '';
		try {
			const payload = buildPayload();
			if (overrideStatus) payload.status = overrideStatus;
			const result = await onSave(payload);
			saveMessage = overrideStatus === 'published' ? 'Published!' : 'Saved!';
			// Visible confirmation that survives the parent's remount
			// (the edit route reassigns `postData` on `onSaved`, which
			// re-keys the PostEditor block — the local `saveMessage` is
			// destroyed with the old instance, so a global toast is the
			// only persistence point that survives the swap).
			if (overrideStatus === 'published') {
				toast.success('Post published');
			} else if (overrideStatus === 'draft') {
				toast.success('Saved as draft');
			} else {
				toast.success('Post saved');
			}
			onSaved?.(result);
			if (mode === 'edit') {
				loadRevisions();
			}
			setTimeout(() => (saveMessage = ''), 3000);
		} catch (e) {
			saveMessage = 'Save failed';
			const message = e instanceof Error ? e.message : 'Save failed';
			toast.error('Save failed', { description: message });
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
			focalX = media.focal_x ?? 0.5;
			focalY = media.focal_y ?? 0.5;
		}
	}

	function setFocalPoint(el: HTMLElement, clientX: number, clientY: number) {
		const rect = el.getBoundingClientRect();
		focalX = Math.round(((clientX - rect.left) / rect.width) * 100) / 100;
		focalY = Math.round(((clientY - rect.top) / rect.height) * 100) / 100;
		if (featuredImageId) {
			// Forensic Wave-2 PR-8 (C-8): the route is mounted under the
			// `/api/admin/blog` nest in `backend/src/handlers/blog.rs`, not
			// `/api/admin/media/...` — the prior URL 404'd silently because
			// `.catch(() => {})` swallowed the error and the focal point
			// never persisted. Surface the failure via the global toast so
			// editors notice instead of saving and reloading to find the
			// crop reset.
			api.put(`/api/admin/blog/media/${featuredImageId}`, {
				focal_x: focalX,
				focal_y: focalY
			}).catch((err: unknown) => {
				const message = err instanceof Error ? err.message : 'Could not save focal point';
				toast.error('Focal point not saved', { description: message });
			});
		}
	}

	function handleFocalClick(e: MouseEvent) {
		setFocalPoint(e.currentTarget as HTMLElement, e.clientX, e.clientY);
	}

	function handleFocalKeydown(e: KeyboardEvent) {
		if (e.key !== 'Enter' && e.key !== ' ') return;
		e.preventDefault();
		const el = e.currentTarget as HTMLElement;
		const rect = el.getBoundingClientRect();
		setFocalPoint(el, rect.left + rect.width / 2, rect.top + rect.height / 2);
	}

	function removeFeaturedImage() {
		featuredImageId = undefined;
		featuredImageUrl = '';
	}

	function toggleCategory(id: string) {
		if (selectedCategoryIds.has(id)) {
			selectedCategoryIds.delete(id);
		} else {
			selectedCategoryIds.add(id);
		}
	}

	function toggleTag(id: string) {
		if (selectedTagIds.has(id)) {
			selectedTagIds.delete(id);
		} else {
			selectedTagIds.add(id);
		}
	}

	async function addCategory() {
		if (!newCategoryName.trim()) return;
		try {
			const cat = await api.post<BlogCategory>('/api/admin/blog/categories', {
				name: newCategoryName
			});
			allCategories = [...allCategories, cat];
			selectedCategoryIds.add(cat.id);
			newCategoryName = '';
		} catch (e) {
			console.error('Failed to create category', e);
		}
	}

	async function addTag() {
		if (!newTagName.trim()) return;
		try {
			const tag = await api.post<BlogTag>('/api/admin/blog/tags', { name: newTagName });
			allTags = [...allTags, tag];
			selectedTagIds.add(tag.id);
			newTagName = '';
		} catch (e) {
			console.error('Failed to create tag', e);
		}
	}

	async function moveToTrash() {
		if (!post || post.status === 'trash') return;
		const ok = await confirmDialog({
			title: 'Move this post to the Trash?',
			message:
				'You can restore it later from Blog → Trash, or delete it permanently from there.',
			confirmLabel: 'Move to trash',
			variant: 'warning'
		});
		if (!ok) return;
		trashing = true;
		saveMessage = '';
		try {
			await api.put(`/api/admin/blog/posts/${post.id}/status`, { status: 'trash' });
			await goto(resolve('/admin/blog'));
		} catch (e) {
			saveMessage = 'Could not move to trash';
			console.error(e);
		} finally {
			trashing = false;
		}
	}

	async function restoreFromTrash() {
		if (!post || post.status !== 'trash') return;
		restoring = true;
		saveMessage = '';
		try {
			await api.post<BlogPostResponse>(`/api/admin/blog/posts/${post.id}/restore`);
			saveMessage = 'Restored from trash';
			await onRestored?.();
			setTimeout(() => (saveMessage = ''), 3000);
		} catch (e) {
			saveMessage = 'Restore failed';
			console.error(e);
		} finally {
			restoring = false;
		}
	}

	async function deletePermanently() {
		if (!post || post.status !== 'trash') return;
		const ok = await confirmDialog({
			title: 'Permanently delete this post?',
			message: 'This cannot be undone. All revisions and metadata will be removed.',
			confirmLabel: 'Delete permanently',
			variant: 'danger'
		});
		if (!ok) return;
		deletingPermanent = true;
		saveMessage = '';
		try {
			await api.delete(`/api/admin/blog/posts/${post.id}`);
			await goto(resolve('/admin/blog'));
		} catch (e) {
			saveMessage = 'Delete failed';
			console.error(e);
		} finally {
			deletingPermanent = false;
		}
	}

	async function restoreRevision(revId: string) {
		if (!post) return;
		const ok = await confirmDialog({
			title: 'Restore this revision?',
			message: 'Your current content will be saved as a new revision before the swap.',
			confirmLabel: 'Restore revision',
			variant: 'warning'
		});
		if (!ok) return;
		try {
			const result = await api.post<BlogPostResponse>(
				`/api/admin/blog/posts/${post.id}/revisions/${revId}/restore`
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
			<a href={resolve('/admin/blog')}>
				<ArrowLeft size={16} weight="bold" />
				<span>Back to posts</span>
			</a>
		</div>

		{#if mode === 'edit' && post?.status === 'trash'}
			<div class="post-editor__trash-banner" role="status">
				<div class="post-editor__trash-banner__text">
					<p class="post-editor__trash-banner__title">This post is in the Trash</p>
					<p class="post-editor__trash-banner__hint">
						Restore it to continue editing and publishing, or delete it permanently.
						Trashed posts are not shown on the public site.
					</p>
					{#if post.pre_trash_status}
						<p class="post-editor__trash-banner__meta">
							Before trash, status was <strong
								>{post.pre_trash_status.replace(/_/g, ' ')}</strong
							>
							{#if post.trashed_at}
								· moved {new Date(post.trashed_at).toLocaleString()}
							{/if}
						</p>
					{/if}
				</div>
				<div class="post-editor__trash-banner__actions">
					<button
						type="button"
						class="post-editor__trash-restore"
						onclick={restoreFromTrash}
						disabled={restoring || deletingPermanent}
					>
						{restoring ? 'Restoring…' : 'Restore'}
					</button>
					<button
						type="button"
						class="post-editor__trash-delete"
						onclick={deletePermanently}
						disabled={restoring || deletingPermanent}
					>
						{deletingPermanent ? 'Deleting…' : 'Delete permanently'}
					</button>
				</div>
			</div>
		{/if}

		<!-- Title -->
		<input
			id="post-title"
			name="post-title"
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
			{autosaveStatus}
			{lastSavedAt}
			{revisions}
			focusKeyword=""
			{metaTitle}
			{metaDescription}
		/>

		<!-- Status bar -->
		<div class="post-editor__statusbar">
			<span
				>{wordCount} words · {charCount} characters · ~{Math.max(
					1,
					Math.ceil(wordCount / 238)
				)} min read</span
			>
			<span
				class="post-editor__autosave"
				class:post-editor__autosave--pending={autosaveStatus === 'pending'}
				class:post-editor__autosave--saving={autosaveStatus === 'saving'}
				class:post-editor__autosave--saved={autosaveStatus === 'saved'}
				class:post-editor__autosave--error={autosaveStatus === 'error'}
			>
				{#if autosaveStatus === 'pending'}
					<span class="autosave-dot autosave-dot--amber"></span> Unsaved changes
				{:else if autosaveStatus === 'saving'}
					<span class="autosave-dot autosave-dot--pulse"></span> Saving draft...
				{:else if autosaveStatus === 'saved'}
					<span class="autosave-dot autosave-dot--green"></span> Saved {formatAutosaveRelative(
						lastSavedAt
					)}
				{:else if autosaveStatus === 'error'}
					<span class="autosave-dot autosave-dot--red"></span> Autosave failed
				{/if}
			</span>
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
						{#if post?.status === 'trash'}
							<button
								class="publish-btn publish-btn--draft publish-btn--full"
								onclick={() => handleSave()}
								disabled={saving}
							>
								<FloppyDisk size={16} weight="bold" />
								<span>Save changes</span>
							</button>
						{:else}
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
								<span
									>{mode === 'edit' && post?.status === 'published'
										? 'Update'
										: 'Publish'}</span
								>
							</button>
						{/if}
					</div>
					{#if mode === 'edit' && post && post.status !== 'trash'}
						<button
							type="button"
							class="post-editor__move-trash"
							onclick={moveToTrash}
							disabled={trashing || saving}
						>
							<Trash size={16} weight="bold" />
							<span>{trashing ? 'Moving…' : 'Move to trash'}</span>
						</button>
					{/if}
				</div>
			</div>

			<!-- Post Format -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Format</h3>
				<div class="sidebar-section__content">
					<label class="sidebar-field">
						<span class="sidebar-field__label">Post Format</span>
						<select
							id="post-format"
							name="post-format"
							class="sidebar-field__select"
							bind:value={postFormat}
						>
							<option value="standard">Standard</option>
							<option value="aside">Aside</option>
							<option value="image">Image</option>
							<option value="video">Video</option>
							<option value="quote">Quote</option>
							<option value="link">Link</option>
							<option value="gallery">Gallery</option>
							<option value="audio">Audio</option>
							<option value="status">Status</option>
							<option value="chat">Chat</option>
						</select>
					</label>
				</div>
			</div>

			<!-- Status & Visibility -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Status & Visibility</h3>
				<div class="sidebar-section__content">
					<label class="sidebar-field">
						<span class="sidebar-field__label">Status</span>
						<select
							id="post-status"
							name="post-status"
							class="sidebar-field__select"
							bind:value={status}
							disabled={post?.status === 'trash'}
						>
							<option value="draft">Draft</option>
							<option value="pending_review">Pending Review</option>
							<option value="published">Published</option>
							<option value="private">Private</option>
							<option value="scheduled">Scheduled</option>
							{#if post?.status === 'trash'}
								<option value="trash">In trash</option>
							{/if}
						</select>
					</label>

					<label class="sidebar-field">
						<span class="sidebar-field__label">Visibility</span>
						<select
							id="post-visibility"
							name="post-visibility"
							class="sidebar-field__select"
							bind:value={visibility}
						>
							<option value="public">Public</option>
							<option value="private">Private</option>
							<option value="password">Password Protected</option>
						</select>
					</label>

					{#if visibility === 'password'}
						<label class="sidebar-field">
							<span class="sidebar-field__label">Post Password</span>
							<input
								id="post-password"
								name="post-password"
								type="password"
								class="sidebar-field__input"
								bind:value={postPassword}
								placeholder="Enter password…"
								autocomplete="new-password"
							/>
						</label>
					{/if}

					{#if status === 'scheduled'}
						<label class="sidebar-field">
							<span class="sidebar-field__label">Schedule for</span>
							<input
								id="post-scheduled-at"
								name="post-scheduled-at"
								type="datetime-local"
								class="sidebar-field__input"
								bind:value={scheduledAt}
							/>
						</label>
					{/if}

					<label class="sidebar-field sidebar-field--inline">
						<input
							id="post-sticky"
							name="post-sticky"
							type="checkbox"
							bind:checked={isSticky}
						/>
						<span>Sticky post</span>
					</label>

					<label class="sidebar-field sidebar-field--inline">
						<input
							id="post-allow-comments"
							name="post-allow-comments"
							type="checkbox"
							bind:checked={allowComments}
						/>
						<span>Allow comments</span>
					</label>
				</div>
			</div>

			<!-- Author -->
			{#if allAdmins.length > 0}
				<div class="sidebar-section">
					<h3 class="sidebar-section__title">Author</h3>
					<div class="sidebar-section__content">
						<label class="sidebar-field">
							<span class="sidebar-field__label">Assign to</span>
							<select
								id="post-author"
								name="post-author"
								class="sidebar-field__select"
								bind:value={authorId}
							>
								<option value="">— Current user —</option>
								{#each allAdmins as user (user.id)}
									<option value={user.id}>{user.name} ({user.role})</option>
								{/each}
							</select>
						</label>
					</div>
				</div>
			{/if}

			<!-- Permalink -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Permalink</h3>
				<div class="sidebar-section__content">
					<label class="sidebar-field">
						<span class="sidebar-field__label">Slug</span>
						<input
							id="post-slug"
							name="post-slug"
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
									id="post-cat-{cat.id}"
									name="post-cat-{cat.id}"
									type="checkbox"
									checked={selectedCategoryIds.has(cat.id)}
									onchange={() => toggleCategory(cat.id)}
								/>
								<span>{cat.name}</span>
							</label>
						{/each}
					</div>
					<div class="add-taxonomy">
						<input
							id="new-category-name"
							name="new-category-name"
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
								class:tag-pill--selected={selectedTagIds.has(tag.id)}
								onclick={() => toggleTag(tag.id)}
							>
								<span>{tag.name}</span>
								{#if selectedTagIds.has(tag.id)}
									<X size={12} weight="bold" />
								{/if}
							</button>
						{/each}
					</div>
					<div class="add-taxonomy">
						<input
							id="new-tag-name"
							name="new-tag-name"
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

			<!-- Custom Fields -->
			{#if mode === 'edit' && post}
				<div class="sidebar-section">
					<h3 class="sidebar-section__title">Custom Fields</h3>
					<div class="sidebar-section__content">
						{#if postMeta.length > 0}
							<ul class="meta-list">
								{#each postMeta as m (m.id)}
									<li class="meta-item">
										<span class="meta-item__key">{m.meta_key}</span>
										<span class="meta-item__value">{m.meta_value}</span>
										<button
											class="meta-item__del"
											title="Delete"
											onclick={() => deleteMeta(m.meta_key)}
										>
											<X size={12} weight="bold" />
										</button>
									</li>
								{/each}
							</ul>
						{/if}
						<div class="meta-add">
							<input
								id="new-meta-key"
								name="new-meta-key"
								type="text"
								class="meta-add__input"
								bind:value={newMetaKey}
								placeholder="Key"
							/>
							<input
								id="new-meta-value"
								name="new-meta-value"
								type="text"
								class="meta-add__input"
								bind:value={newMetaValue}
								placeholder="Value"
								onkeydown={(e) => e.key === 'Enter' && addMeta()}
							/>
							<button class="add-taxonomy__btn" onclick={addMeta} title="Add field">
								<Plus size={14} weight="bold" />
							</button>
						</div>
					</div>
				</div>
			{/if}

			<!-- Featured Image -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Featured Image</h3>
				<div class="sidebar-section__content">
					{#if featuredImageUrl}
						<div class="featured-preview">
							<div
								class="focal-picker"
								role="button"
								tabindex="0"
								onclick={handleFocalClick}
								onkeydown={handleFocalKeydown}
								title="Click to set focal point"
							>
								<img
									src={featuredImageUrl}
									alt="Featured"
									style="object-position: {focalX * 100}% {focalY * 100}%"
								/>
								<span
									class="focal-dot"
									style="left:{focalX * 100}%;top:{focalY * 100}%"
								></span>
							</div>
							<p class="focal-hint">Click image to set focal point</p>
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
						id="post-excerpt"
						name="post-excerpt"
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
							id="post-meta-title"
							name="post-meta-title"
							type="text"
							class="sidebar-field__input"
							bind:value={metaTitle}
							placeholder="SEO title"
							maxlength="60"
						/>
					</label>

					<label class="sidebar-field">
						<span class="sidebar-field__label">
							Meta description <span class="char-count"
								>({metaDescription.length}/160)</span
							>
						</span>
						<textarea
							id="post-meta-description"
							name="post-meta-description"
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
							id="post-canonical-url"
							name="post-canonical-url"
							type="url"
							class="sidebar-field__input"
							bind:value={canonicalUrl}
							placeholder="https://..."
						/>
					</label>

					<label class="sidebar-field">
						<span class="sidebar-field__label">OG Image URL</span>
						<input
							id="post-og-image"
							name="post-og-image"
							type="url"
							class="sidebar-field__input"
							bind:value={ogImageUrl}
							placeholder="https://..."
						/>
					</label>
				</div>
			</div>

			<!-- SEO Analysis -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">SEO Analysis</h3>
				<div class="sidebar-section__content">
					<ul class="seo-list">
						{#each seoChecks as c (c.label)}
							<li
								class={[
									'seo-item',
									{
										'seo-item--pass': c.pass,
										'seo-item--warn': !c.pass && c.warn,
										'seo-item--fail': !c.pass && !c.warn
									}
								]}
							>
								<span class="seo-item__dot"></span>
								<span class="seo-item__label">{c.label}</span>
								{#if c.value}<span class="seo-item__value">{c.value}</span>{/if}
							</li>
						{/each}
					</ul>
				</div>
			</div>

			<!-- Social Card Preview -->
			<div class="sidebar-section">
				<h3 class="sidebar-section__title">Social Card Preview</h3>
				<div class="sidebar-section__content">
					<div class="social-card">
						{#if featuredImageUrl || ogImageUrl}
							<img
								class="social-card__img"
								src={ogImageUrl || featuredImageUrl}
								alt=""
							/>
						{:else}
							<div class="social-card__placeholder">No image set</div>
						{/if}
						<div class="social-card__body">
							<p class="social-card__domain">yoursite.com</p>
							<p class="social-card__title">{metaTitle || title || 'Post title'}</p>
							<p class="social-card__desc">
								{metaDescription || excerpt || 'Post description will appear here.'}
							</p>
						</div>
					</div>
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
										<span class="revision-item__number"
											>#{rev.revision_number}</span
										>
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
									<button
										class="revision-item__restore"
										onclick={() => restoreRevision(rev.id)}
									>
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

	.post-editor__autosave {
		font-size: 0.75rem;
		color: transparent;
		transition: color 0.2s;
	}

	.post-editor__autosave--pending {
		color: var(--color-grey-500, #64748b);
	}

	.post-editor__autosave--saving {
		color: var(--color-grey-400, #94a3b8);
	}

	.post-editor__autosave--saved {
		color: #22c55e;
	}

	.post-editor__autosave--error {
		color: #ef4444;
	}

	.autosave-dot {
		display: inline-block;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-grey-500, #475569);
		vertical-align: middle;
		margin-right: 0.2rem;
	}

	.autosave-dot--green {
		background: #22c55e;
		box-shadow: 0 0 4px rgba(34, 197, 94, 0.5);
	}

	.autosave-dot--amber {
		background: #f59e0b;
		box-shadow: 0 0 4px rgba(245, 158, 11, 0.5);
	}

	.autosave-dot--red {
		background: #ef4444;
		box-shadow: 0 0 4px rgba(239, 68, 68, 0.5);
	}

	.autosave-dot--pulse {
		background: var(--color-grey-400, #94a3b8);
		animation: pulse-dot 1.2s ease-in-out infinite;
	}

	@keyframes pulse-dot {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.3;
		}
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

	.publish-btn--full {
		flex: 1 1 100%;
	}

	.post-editor__move-trash {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.35rem;
		width: 100%;
		margin-top: 0.5rem;
		padding: 0.45rem 0.5rem;
		border: 1px solid rgba(239, 68, 68, 0.35);
		border-radius: 0.375rem;
		background: transparent;
		color: #f87171;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
	}

	.post-editor__move-trash:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.1);
	}

	.post-editor__move-trash:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.post-editor__trash-banner {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
		padding: 1rem 1.1rem;
		border-radius: 0.5rem;
		border: 1px solid rgba(239, 68, 68, 0.35);
		background: rgba(239, 68, 68, 0.08);
	}

	.post-editor__trash-banner__title {
		margin: 0 0 0.35rem;
		font-size: 0.95rem;
		font-weight: 700;
		color: #fecaca;
	}

	.post-editor__trash-banner__hint {
		margin: 0;
		font-size: 0.8rem;
		color: var(--color-grey-300, #cbd5e1);
		line-height: 1.45;
		max-width: 40rem;
	}

	.post-editor__trash-banner__meta {
		margin: 0.5rem 0 0;
		font-size: 0.72rem;
		color: var(--color-grey-400, #94a3b8);
		text-transform: capitalize;
	}

	.post-editor__trash-banner__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		flex-shrink: 0;
	}

	.post-editor__trash-restore {
		padding: 0.45rem 0.9rem;
		border: none;
		border-radius: 0.375rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}

	.post-editor__trash-restore:hover:not(:disabled) {
		opacity: 0.92;
	}

	.post-editor__trash-delete {
		padding: 0.45rem 0.9rem;
		border: 1px solid rgba(239, 68, 68, 0.5);
		border-radius: 0.375rem;
		background: rgba(239, 68, 68, 0.15);
		color: #fecaca;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}

	.post-editor__trash-delete:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.25);
	}

	.post-editor__trash-restore:disabled,
	.post-editor__trash-delete:disabled {
		opacity: 0.45;
		cursor: not-allowed;
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

	/* Focal point picker */
	.focal-picker {
		position: relative;
		cursor: crosshair;
		border-radius: 0.375rem;
		overflow: hidden;
		aspect-ratio: 16 / 9;
	}

	.focal-picker img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
		pointer-events: none;
	}

	.focal-dot {
		position: absolute;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: rgba(15, 164, 175, 0.9);
		border: 2px solid #fff;
		box-shadow: 0 0 0 2px rgba(15, 164, 175, 0.5);
		transform: translate(-50%, -50%);
		pointer-events: none;
	}

	.focal-hint {
		font-size: 0.65rem;
		color: var(--color-grey-500, #475569);
		margin: 0.25rem 0 0;
		text-align: center;
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

	/* Custom Fields */
	.meta-list {
		list-style: none;
		padding: 0;
		margin: 0 0 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.meta-item {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.75rem;
		padding: 0.2rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.meta-item__key {
		font-weight: 600;
		color: var(--color-teal-light, #15c5d1);
		min-width: 4rem;
		flex-shrink: 0;
	}

	.meta-item__value {
		flex: 1;
		color: var(--color-grey-300, #cbd5e1);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.meta-item__del {
		background: transparent;
		border: none;
		color: var(--color-grey-500, #475569);
		cursor: pointer;
		padding: 0.1rem;
		display: flex;
		border-radius: 0.2rem;
		flex-shrink: 0;
	}

	.meta-item__del:hover {
		color: #ef4444;
	}

	.meta-add {
		display: flex;
		gap: 0.3rem;
		align-items: center;
	}

	.meta-add__input {
		flex: 1;
		min-width: 0;
		padding: 0.3rem 0.5rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.3rem;
		color: var(--color-grey-200, #e2e8f0);
		font-size: 0.75rem;
		outline: none;
	}

	.meta-add__input:focus {
		border-color: rgba(15, 164, 175, 0.5);
	}

	/* SEO Analysis */
	.seo-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.seo-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: var(--color-grey-400, #94a3b8);
	}

	.seo-item__dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
		background: rgba(255, 255, 255, 0.2);
	}

	.seo-item--pass .seo-item__dot {
		background: #22c55e;
	}
	.seo-item--pass {
		color: var(--color-grey-300, #cbd5e1);
	}
	.seo-item--warn .seo-item__dot {
		background: #f59e0b;
	}
	.seo-item--warn {
		color: #f59e0b;
	}
	.seo-item--fail .seo-item__dot {
		background: rgba(255, 255, 255, 0.15);
	}

	.seo-item__label {
		flex: 1;
	}
	.seo-item__value {
		font-size: 0.7rem;
		color: var(--color-grey-500, #475569);
		white-space: nowrap;
	}

	/* Social Card Preview */
	.social-card {
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.5rem;
		overflow: hidden;
		font-size: 0.75rem;
	}

	.social-card__img {
		width: 100%;
		aspect-ratio: 1.91 / 1;
		object-fit: cover;
		display: block;
	}

	.social-card__placeholder {
		width: 100%;
		aspect-ratio: 1.91 / 1;
		background: rgba(255, 255, 255, 0.04);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-grey-500, #475569);
		font-size: 0.7rem;
	}

	.social-card__body {
		padding: 0.6rem 0.75rem;
		background: rgba(255, 255, 255, 0.03);
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.social-card__domain {
		font-size: 0.65rem;
		color: var(--color-grey-500, #475569);
		text-transform: uppercase;
		letter-spacing: 0.03em;
		margin: 0;
	}

	.social-card__title {
		font-weight: 600;
		color: var(--color-white, #fff);
		margin: 0;
		line-height: 1.3;
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
	}

	.social-card__desc {
		color: var(--color-grey-400, #94a3b8);
		margin: 0;
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
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
