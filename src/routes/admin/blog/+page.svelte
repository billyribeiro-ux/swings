<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import type {
		BlogPostListItem,
		PaginatedResponse,
		PostStatus,
		UserResponse
	} from '$lib/api/types';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import ArticleIcon from 'phosphor-svelte/lib/ArticleIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import { confirmDialog } from '$lib/stores/confirm.svelte';
	import { toast } from '$lib/stores/toast.svelte';

	let posts: BlogPostListItem[] = $state([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let statusFilter: PostStatus | '' = $state('');
	let search = $state('');

	// Bulk selection
	let selectedIds: string[] = $state([]);
	let bulkActionValue = $state('');
	const allSelected = $derived(
		posts.length > 0 && posts.every((p) => selectedIds.includes(p.id))
	);

	function toggleSelect(id: string) {
		if (selectedIds.includes(id)) {
			selectedIds = selectedIds.filter((x) => x !== id);
		} else {
			selectedIds = [...selectedIds, id];
		}
	}

	function toggleSelectAll() {
		if (allSelected) {
			selectedIds = [];
		} else {
			selectedIds = posts.map((p) => p.id);
		}
	}

	async function executeBulkAction() {
		if (!bulkActionValue || selectedIds.length === 0) return;
		const action = bulkActionValue;
		bulkActionValue = '';
		const count = selectedIds.length;

		if (action === 'delete') {
			if (statusFilter !== 'trash') {
				toast.warning('Permanent deletion only applies to posts in the Trash', {
					description: 'Open the Trash tab, select posts, then choose Delete permanently.'
				});
				return;
			}
			const ok = await confirmDialog({
				title: `Permanently delete ${count} post${count === 1 ? '' : 's'}?`,
				message:
					'This action cannot be undone. The posts and their revision history will be removed.',
				confirmLabel: 'Delete permanently',
				variant: 'danger'
			});
			if (!ok) return;
			const results = await Promise.allSettled(
				selectedIds.map((id) => api.delete(`/api/admin/blog/posts/${id}`))
			);
			const failed = results.filter((r) => r.status === 'rejected').length;
			if (failed === 0) toast.success(`Deleted ${count} post${count === 1 ? '' : 's'}`);
			else if (failed === count)
				toast.error(`Failed to delete ${count} post${count === 1 ? '' : 's'}`);
			else
				toast.warning(`Deleted ${count - failed} of ${count}`, {
					description: `${failed} failed`
				});
		} else {
			const status =
				action === 'trash' ? 'trash' : action === 'publish' ? 'published' : 'draft';
			const results = await Promise.allSettled(
				selectedIds.map((id) => api.put(`/api/admin/blog/posts/${id}/status`, { status }))
			);
			const failed = results.filter((r) => r.status === 'rejected').length;
			const verb =
				status === 'trash'
					? 'Moved to trash'
					: status === 'published'
						? 'Published'
						: 'Saved as draft';
			if (failed === 0) toast.success(`${verb} · ${count} post${count === 1 ? '' : 's'}`);
			else if (failed === count) toast.error(`Bulk ${action} failed`);
			else
				toast.warning(`${verb} ${count - failed} of ${count}`, {
					description: `${failed} failed`
				});
		}
		selectedIds = [];
		loadPosts();
	}
	let searchTimeout: ReturnType<typeof setTimeout>;

	onMount(loadPosts);

	async function loadPosts() {
		loading = true;
		try {
			let url = `/api/admin/blog/posts?page=${page}&per_page=20`;
			if (statusFilter) url += `&status=${statusFilter}`;
			if (search) url += `&search=${encodeURIComponent(search)}`;
			const res = await api.get<PaginatedResponse<BlogPostListItem>>(url);
			posts = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch (e) {
			toast.error('Failed to load posts', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			loading = false;
		}
	}

	function handleSearch(e: Event) {
		const val = (e.target as HTMLInputElement).value;
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			search = val;
			page = 1;
			loadPosts();
		}, 300);
	}

	function changeStatus(s: PostStatus | '') {
		statusFilter = s;
		page = 1;
		loadPosts();
	}

	async function deletePost(id: string) {
		const ok = await confirmDialog({
			title: 'Move this post to trash?',
			message: 'You can restore it from the Trash tab before it is permanently removed.',
			confirmLabel: 'Move to trash',
			variant: 'warning'
		});
		if (!ok) return;
		try {
			await api.put(`/api/admin/blog/posts/${id}/status`, { status: 'trash' });
			toast.success('Post moved to trash');
			loadPosts();
		} catch (e) {
			toast.error('Failed to trash post', {
				description: e instanceof Error ? e.message : undefined
			});
		}
	}

	async function hardDelete(id: string) {
		const ok = await confirmDialog({
			title: 'Permanently delete this post?',
			message: 'This cannot be undone. The post and its revision history will be removed.',
			confirmLabel: 'Delete permanently',
			variant: 'danger'
		});
		if (!ok) return;
		try {
			await api.delete(`/api/admin/blog/posts/${id}`);
			toast.success('Post permanently deleted');
			loadPosts();
		} catch (e) {
			toast.error('Failed to delete post', {
				description: e instanceof Error ? e.message : undefined
			});
		}
	}

	async function restorePost(id: string) {
		try {
			await api.post(`/api/admin/blog/posts/${id}/restore`);
			toast.success('Post restored');
			loadPosts();
		} catch (e) {
			toast.error('Failed to restore post', {
				description: e instanceof Error ? e.message : undefined
			});
		}
	}

	function statusBadge(status: PostStatus): string {
		const map: Record<string, string> = {
			draft: 'badge--draft',
			pending_review: 'badge--pending',
			published: 'badge--published',
			private: 'badge--private',
			scheduled: 'badge--scheduled',
			trash: 'badge--trash'
		};
		return map[status] || '';
	}

	function statusLabel(status: PostStatus): string {
		const map: Record<string, string> = {
			draft: 'Draft',
			pending_review: 'Pending',
			published: 'Published',
			private: 'Private',
			scheduled: 'Scheduled',
			trash: 'Trash'
		};
		return map[status] || status;
	}

	function formatDate(d: string | null): string {
		if (!d) return '—';
		return new Date(d).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	const STATUS_TABS: { value: PostStatus | ''; label: string }[] = [
		{ value: '', label: 'All' },
		{ value: 'published', label: 'Published' },
		{ value: 'draft', label: 'Drafts' },
		{ value: 'pending_review', label: 'Pending' },
		{ value: 'scheduled', label: 'Scheduled' },
		{ value: 'trash', label: 'Trash' }
	];

	// Quick Edit
	let qePostId: string | null = $state(null);
	let qeTitle = $state('');
	let qeStatus: PostStatus = $state('draft');
	let qeAuthorId = $state('');
	let qeSaving = $state(false);
	let admins: UserResponse[] = $state([]);

	async function loadAdmins() {
		if (admins.length > 0) return;
		try {
			const res = await api.get<UserResponse[]>('/api/admin/members?role=admin&per_page=50');
			admins = Array.isArray(res) ? res : [];
		} catch {
			admins = [];
		}
	}

	function openQuickEdit(post: BlogPostListItem) {
		qePostId = post.id;
		qeTitle = post.title;
		qeStatus = post.status;
		qeAuthorId = post.author_id || '';
		loadAdmins();
	}

	function closeQuickEdit() {
		qePostId = null;
	}

	async function saveQuickEdit() {
		if (!qePostId) return;
		qeSaving = true;
		try {
			await api.put(`/api/admin/blog/posts/${qePostId}`, {
				title: qeTitle,
				status: qeStatus,
				author_id: qeAuthorId || undefined
			});
			closeQuickEdit();
			toast.success('Post updated');
			loadPosts();
		} catch (e) {
			toast.error('Quick edit save failed', {
				description: e instanceof Error ? e.message : undefined
			});
		} finally {
			qeSaving = false;
		}
	}
</script>

<svelte:head>
	<title>Blog Posts — Admin</title>
</svelte:head>

<div class="blog-admin">
	<header class="blog-admin__page-header">
		<div class="blog-admin__heading">
			<span class="blog-admin__eyebrow">Content</span>
			<h1 class="blog-admin__title">Blog Posts</h1>
			<p class="blog-admin__subtitle">
				Author, schedule, and manage every post that appears on the public blog.
			</p>
		</div>
		<a href={resolve('/admin/blog/new')} class="blog-admin__cta">
			<PlusIcon size={16} weight="bold" />
			<span>New post</span>
		</a>
	</header>

	<!-- Filters -->
	<div class="blog-admin__toolbar">
		<div class="blog-admin__tabs" role="tablist" aria-label="Filter by status">
			{#each STATUS_TABS as tab (tab.value)}
				<button
					class="blog-admin__tab"
					class:blog-admin__tab--active={statusFilter === tab.value}
					onclick={() => changeStatus(tab.value)}
					role="tab"
					aria-selected={statusFilter === tab.value}
				>
					{tab.label}
				</button>
			{/each}
		</div>

		<div class="blog-admin__search">
			<MagnifyingGlassIcon size={16} weight="bold" class="blog-admin__search-icon" />
			<input
				id="post-search"
				name="search"
				type="search"
				class="blog-admin__search-input"
				placeholder="Search posts…"
				aria-label="Search posts"
				oninput={handleSearch}
			/>
		</div>
	</div>

	<!-- Bulk action bar -->
	{#if selectedIds.length > 0}
		<div class="bulk-bar">
			<span class="bulk-bar__count">{selectedIds.length} selected</span>
			<select class="bulk-bar__select" bind:value={bulkActionValue} aria-label="Bulk action">
				<option value="">Bulk actions…</option>
				<option value="publish">Publish</option>
				<option value="draft">Set to Draft</option>
				<option value="trash">Move to Trash</option>
				<option value="delete">Delete Permanently</option>
			</select>
			<button class="bulk-bar__apply" onclick={executeBulkAction} disabled={!bulkActionValue}>
				Apply
			</button>
			<button
				class="bulk-bar__clear"
				onclick={() => (selectedIds = [])}
				aria-label="Clear selection"
			>
				<XIcon size={14} weight="bold" />
			</button>
		</div>
	{/if}

	<!-- Posts table -->
	{#if loading}
		<div class="blog-admin__skeleton" aria-hidden="true">
			{#each Array(5) as _, i (i)}
				<div class="blog-admin__skeleton-row"></div>
			{/each}
		</div>
	{:else if posts.length === 0}
		<div class="blog-admin__empty">
			<ArticleIcon size={48} weight="duotone" color="var(--color-grey-500)" />
			<p class="blog-admin__empty-title">No posts found</p>
			<p class="blog-admin__empty-body">
				{search || statusFilter
					? 'Try adjusting your search or filters.'
					: 'Get started by creating your first post.'}
			</p>
			{#if !search && !statusFilter}
				<a href={resolve('/admin/blog/new')} class="blog-admin__cta blog-admin__cta--empty">
					<PlusIcon size={16} weight="bold" />
					<span>New post</span>
				</a>
			{/if}
		</div>
	{:else}
		<!-- Mobile: Card view -->
		<div class="blog-admin__cards">
			{#each posts as post (post.id)}
				<div class="post-card">
					<div class="post-card__header">
						<a href={resolve(`/admin/blog/${post.id}`)} class="post-card__title"
							>{post.title}</a
						>
						<span class="badge {statusBadge(post.status)}">
							{statusLabel(post.status)}
						</span>
					</div>
					{#if post.is_sticky}
						<span class="sticky-badge sticky-badge--card">Sticky</span>
					{/if}
					<div class="post-card__row">
						<span class="post-card__label">Author</span>
						<span class="post-card__value">{post.author_name}</span>
					</div>
					<div class="post-card__row">
						<span class="post-card__label">Date</span>
						<span class="post-card__value">
							{#if post.published_at}
								{formatDate(post.published_at)}
							{:else}
								{formatDate(post.created_at)}
							{/if}
						</span>
					</div>
					{#if post.categories.length > 0}
						<div class="post-card__row">
							<span class="post-card__label">Categories</span>
							<span class="post-card__cats">
								{#each post.categories as cat (cat.id)}
									<span class="cat-pill">{cat.name}</span>
								{/each}
							</span>
						</div>
					{/if}
					<div class="post-card__meta">
						{post.word_count} words · {post.reading_time_minutes} min read
					</div>
					<div class="post-card__actions">
						<a
							href={resolve(`/admin/blog/${post.id}`)}
							class="post-card__btn post-card__btn--edit"
						>
							<PencilSimpleIcon size={14} weight="bold" />
							<span>Edit</span>
						</a>
						{#if post.status === 'trash'}
							<button
								class="post-card__btn post-card__btn--edit"
								onclick={() => restorePost(post.id)}
							>
								<span>Restore</span>
							</button>
							<button
								class="post-card__btn post-card__btn--delete"
								onclick={() => hardDelete(post.id)}
							>
								<span>Delete</span>
							</button>
						{:else}
							<button
								class="post-card__btn post-card__btn--delete"
								onclick={() => deletePost(post.id)}
							>
								<span>Trash</span>
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
		<!-- Tablet+: Table view -->
		<div class="blog-admin__table-wrap">
			<table class="blog-admin__table">
				<thead>
					<tr>
						<th class="th-check">
							<input
								type="checkbox"
								checked={allSelected}
								onchange={toggleSelectAll}
								aria-label="Select all posts"
							/>
						</th>
						<th>Title</th>
						<th>Author</th>
						<th>Categories</th>
						<th>Status</th>
						<th>Date</th>
						<th class="th-actions">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each posts as post (post.id)}
						<tr class:tr--selected={selectedIds.includes(post.id)}>
							<td class="td-check">
								<input
									type="checkbox"
									checked={selectedIds.includes(post.id)}
									onchange={() => toggleSelect(post.id)}
									aria-label="Select post"
								/>
							</td>
							<td>
								<a href={resolve(`/admin/blog/${post.id}`)} class="post-title-link">
									{post.title}
								</a>
								{#if post.is_sticky}<span class="sticky-badge">Sticky</span>{/if}
								<div class="post-meta-row">
									{post.word_count} words · {post.reading_time_minutes} min read
								</div>
							</td>
							<td class="td-author">{post.author_name}</td>
							<td class="td-cats">
								{#each post.categories as cat (cat.id)}
									<span class="cat-pill">{cat.name}</span>
								{/each}
							</td>
							<td>
								<span class="badge {statusBadge(post.status)}">
									{statusLabel(post.status)}
								</span>
							</td>
							<td class="td-date">
								{#if post.published_at}
									{formatDate(post.published_at)}
								{:else}
									{formatDate(post.created_at)}
								{/if}
							</td>
							<td class="td-actions">
								<a href={resolve(`/admin/blog/${post.id}`)} class="action-link"
									>Edit</a
								>
								<button
									class="action-btn"
									class:action-btn--active={qePostId === post.id}
									onclick={() =>
										qePostId === post.id
											? closeQuickEdit()
											: openQuickEdit(post)}
								>
									Quick Edit
								</button>
								{#if post.status === 'trash'}
									<button class="action-btn" onclick={() => restorePost(post.id)}
										>Restore</button
									>
									<button
										class="action-btn action-btn--danger"
										onclick={() => hardDelete(post.id)}
									>
										Delete permanently
									</button>
								{:else}
									<button
										class="action-btn action-btn--danger"
										onclick={() => deletePost(post.id)}
									>
										Trash
									</button>
								{/if}
							</td>
						</tr>
						{#if qePostId === post.id}
							<tr class="qe-row">
								<td colspan="7" class="qe-cell">
									<div class="qe-form">
										<div class="qe-form__fields">
											<label class="qe-label" for={`qe-title-${post.id}`}>
												<span class="qe-label__text">Title</span>
												<input
													id={`qe-title-${post.id}`}
													name="qe-title"
													class="qe-input"
													type="text"
													bind:value={qeTitle}
												/>
											</label>
											<label class="qe-label" for={`qe-status-${post.id}`}>
												<span class="qe-label__text">Status</span>
												<select
													id={`qe-status-${post.id}`}
													name="qe-status"
													class="qe-select"
													bind:value={qeStatus}
												>
													<option value="draft">Draft</option>
													<option value="pending_review"
														>Pending Review</option
													>
													<option value="published">Published</option>
													<option value="private">Private</option>
													<option value="scheduled">Scheduled</option>
												</select>
											</label>
											{#if admins.length > 0}
												<label
													class="qe-label"
													for={`qe-author-${post.id}`}
												>
													<span class="qe-label__text">Author</span>
													<select
														id={`qe-author-${post.id}`}
														name="qe-author"
														class="qe-select"
														bind:value={qeAuthorId}
													>
														{#each admins as a (a.id)}
															<option value={a.id}
																>{a.name || a.email}</option
															>
														{/each}
													</select>
												</label>
											{/if}
										</div>
										<div class="qe-form__actions">
											<button
												class="qe-save"
												onclick={saveQuickEdit}
												disabled={qeSaving}
											>
												{qeSaving ? 'Saving…' : 'Update'}
											</button>
											<button class="qe-cancel" onclick={closeQuickEdit}
												>Cancel</button
											>
										</div>
									</div>
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Pagination -->
		{#if totalPages > 1}
			<div class="blog-admin__pagination">
				<button
					disabled={page <= 1}
					onclick={() => {
						page--;
						loadPosts();
					}}
					class="blog-admin__page-btn"
					aria-label="Previous page"
				>
					<CaretLeftIcon size={16} weight="bold" />
					<span>Prev</span>
				</button>
				<span class="blog-admin__page-info"
					>Page {page} of {totalPages} · {total} {total === 1 ? 'post' : 'posts'}</span
				>
				<button
					disabled={page >= totalPages}
					onclick={() => {
						page++;
						loadPosts();
					}}
					class="blog-admin__page-btn"
					aria-label="Next page"
				>
					<span>Next</span>
					<CaretRightIcon size={16} weight="bold" />
				</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	@keyframes shimmer {
		0% {
			background-position: -200% 0;
		}
		100% {
			background-position: 200% 0;
		}
	}

	/* ====================================================================
	   PAGE HEADER
	   ==================================================================== */
	.blog-admin {
		max-width: 100%;
	}

	.blog-admin__page-header {
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		margin-bottom: 1.25rem;
	}

	.blog-admin__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.blog-admin__title {
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.2;
		letter-spacing: -0.01em;
		margin: 0.25rem 0 0.4rem;
	}

	.blog-admin__subtitle {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
		margin: 0;
	}

	.blog-admin__cta {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 3rem;
		padding: 0.65rem 1rem;
		border-radius: var(--radius-2xl);
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		color: var(--color-white);
		font-weight: 600;
		font-size: 0.875rem;
		text-decoration: none;
		border: none;
		cursor: pointer;
		align-self: flex-start;
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
		transition: all 200ms var(--ease-out);
	}

	.blog-admin__cta:hover {
		background: linear-gradient(135deg, var(--color-teal-light), var(--color-teal));
		transform: translateY(-1px);
		box-shadow: 0 8px 20px -4px rgba(15, 164, 175, 0.55);
	}

	.blog-admin__cta--empty {
		margin-top: 0.75rem;
	}

	/* ====================================================================
	   TOOLBAR — tabs + search
	   ==================================================================== */
	.blog-admin__toolbar {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		margin-bottom: 1rem;
	}

	.blog-admin__tabs {
		display: inline-flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		padding: 0.25rem;
		background-color: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
	}

	.blog-admin__tab {
		padding: 0.45rem 0.75rem;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-400);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}

	.blog-admin__tab:hover {
		color: var(--color-white);
	}

	.blog-admin__tab--active {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	.blog-admin__search {
		position: relative;
		display: flex;
		align-items: center;
	}

	.blog-admin__search :global(.blog-admin__search-icon) {
		position: absolute;
		left: 0.875rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	.blog-admin__search-input {
		width: 100%;
		min-height: 3rem;
		padding: 0.65rem 0.875rem 0.65rem 2.4rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: 0.875rem;
		outline: none;
		transition:
			border-color 150ms,
			box-shadow 150ms;
	}

	.blog-admin__search-input::placeholder {
		color: var(--color-grey-500);
	}

	.blog-admin__search-input:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	/* ====================================================================
	   SKELETON / EMPTY STATE
	   ==================================================================== */
	.blog-admin__skeleton {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.blog-admin__skeleton-row {
		height: 80px;
		border-radius: var(--radius-2xl);
		background: linear-gradient(
			90deg,
			rgba(255, 255, 255, 0.03) 0%,
			rgba(255, 255, 255, 0.06) 50%,
			rgba(255, 255, 255, 0.03) 100%
		);
		background-size: 200% 100%;
		animation: shimmer 1.6s ease-in-out infinite;
	}

	.blog-admin__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 3rem 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		text-align: center;
	}

	.blog-admin__empty-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0.5rem 0 0;
	}

	.blog-admin__empty-body {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		margin: 0;
		max-width: 36ch;
	}

	/* ====================================================================
	   MOBILE — card view
	   ==================================================================== */
	.blog-admin__cards {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.blog-admin__table-wrap {
		display: none;
	}

	.post-card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 1rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		transition: all 200ms var(--ease-out);
	}

	.post-card:hover {
		border-color: rgba(255, 255, 255, 0.1);
		transform: translateY(-1px);
	}

	.post-card__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.post-card__title {
		color: var(--color-white);
		font-weight: 600;
		font-size: 1rem;
		text-decoration: none;
		flex: 1;
		line-height: 1.3;
	}

	.post-card__title:hover {
		color: var(--color-teal-light);
	}

	.post-card__row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.post-card__label {
		font-size: 0.75rem;
		color: var(--color-grey-500);
		font-weight: 500;
	}

	.post-card__value {
		font-size: 0.875rem;
		color: var(--color-grey-300);
		text-align: right;
	}

	.post-card__cats {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		justify-content: flex-end;
	}

	.post-card__meta {
		font-size: 0.75rem;
		color: var(--color-grey-400);
	}

	.post-card__actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
		padding-top: 0.625rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.post-card__btn {
		flex: 1;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		min-height: 2.25rem;
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius-2xl);
		font-size: 0.75rem;
		font-weight: 600;
		text-decoration: none;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 150ms var(--ease-out);
	}

	.post-card__btn--edit {
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
	}

	.post-card__btn--edit:hover {
		background-color: rgba(15, 164, 175, 0.22);
	}

	.post-card__btn--delete {
		background-color: rgba(239, 68, 68, 0.08);
		color: #ef4444;
	}

	.post-card__btn--delete:hover {
		background-color: rgba(239, 68, 68, 0.2);
	}

	.sticky-badge--card {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-md);
		background: rgba(234, 179, 8, 0.15);
		color: #eab308;
		font-size: 0.75rem;
		font-weight: 600;
		margin-top: 0.25rem;
		align-self: flex-start;
	}

	.cat-pill {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
		font-size: 0.75rem;
		font-weight: 500;
	}

	.badge {
		display: inline-block;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		font-size: 0.75rem;
		font-weight: 600;
		white-space: nowrap;
		text-transform: capitalize;
		letter-spacing: 0.01em;
	}

	.badge--draft {
		background: rgba(148, 163, 184, 0.15);
		color: #94a3b8;
	}
	.badge--pending {
		background: rgba(234, 179, 8, 0.15);
		color: #eab308;
	}
	.badge--published {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}
	.badge--private {
		background: rgba(139, 92, 246, 0.15);
		color: #8b5cf6;
	}
	.badge--scheduled {
		background: rgba(59, 130, 246, 0.15);
		color: #3b82f6;
	}
	.badge--trash {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	/* ====================================================================
	   PAGINATION
	   ==================================================================== */
	.blog-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1.25rem;
		flex-wrap: wrap;
	}

	.blog-admin__page-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		min-height: 2.25rem;
		padding: 0.5rem 0.875rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}

	.blog-admin__page-btn:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.08);
		border-color: rgba(15, 164, 175, 0.4);
	}

	.blog-admin__page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.blog-admin__page-info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
	}

	/* ====================================================================
	   TABLET+ (>=768px) — table view
	   ==================================================================== */
	@media (min-width: 768px) {
		.blog-admin__page-header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
			margin-bottom: 1.5rem;
		}

		.blog-admin__cta {
			align-self: flex-end;
		}

		.blog-admin__toolbar {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
			gap: 1rem;
			margin-bottom: 1.25rem;
		}

		.blog-admin__search {
			max-width: 22rem;
			flex: 1;
		}

		.blog-admin__cards {
			display: none;
		}

		.blog-admin__table-wrap {
			display: block;
			overflow-x: auto;
			background: rgba(19, 43, 80, 0.35);
			backdrop-filter: blur(24px);
			-webkit-backdrop-filter: blur(24px);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-2xl);
			box-shadow:
				0 1px 0 rgba(255, 255, 255, 0.03) inset,
				0 12px 32px rgba(0, 0, 0, 0.18);
		}

		.blog-admin__table {
			width: 100%;
			border-collapse: collapse;
		}

		.blog-admin__table thead {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.blog-admin__table th {
			text-align: left;
			padding: 0.875rem 1rem;
			font-size: 0.6875rem;
			font-weight: 600;
			text-transform: uppercase;
			letter-spacing: 0.05em;
			color: var(--color-grey-500);
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.th-actions {
			text-align: right;
		}

		.blog-admin__table td {
			padding: 0.875rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			font-size: 0.875rem;
			color: var(--color-grey-300);
			vertical-align: top;
		}

		.blog-admin__table tbody tr {
			transition: background-color 150ms var(--ease-out);
		}

		.blog-admin__table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.blog-admin__table tbody tr:last-child td {
			border-bottom: none;
		}

		.post-title-link {
			color: var(--color-white);
			font-weight: 600;
			text-decoration: none;
		}

		.post-title-link:hover {
			color: var(--color-teal-light);
		}

		.post-meta-row {
			font-size: 0.75rem;
			color: var(--color-grey-500);
			margin-top: 0.25rem;
		}

		.sticky-badge {
			display: inline-block;
			padding: 0.1rem 0.5rem;
			border-radius: var(--radius-full);
			background: rgba(234, 179, 8, 0.15);
			color: #eab308;
			font-size: 0.75rem;
			font-weight: 600;
			margin-left: 0.5rem;
			vertical-align: middle;
		}

		.td-author {
			white-space: nowrap;
			color: var(--color-grey-400);
		}

		.td-date {
			white-space: nowrap;
			color: var(--color-grey-400);
		}

		.td-cats {
			max-width: 14rem;
		}

		.td-cats .cat-pill + .cat-pill {
			margin-left: 0.25rem;
		}

		.td-actions {
			white-space: nowrap;
			text-align: right;
		}

		.action-link {
			color: var(--color-teal-light);
			text-decoration: none;
			font-size: 0.75rem;
			font-weight: 600;
			margin-right: 0.75rem;
			transition: color 150ms var(--ease-out);
		}

		.action-link:hover {
			color: var(--color-teal);
			text-decoration: underline;
		}

		.action-btn {
			border: none;
			background: none;
			font-size: 0.75rem;
			font-weight: 600;
			color: var(--color-grey-300);
			cursor: pointer;
			padding: 0.2rem 0.4rem;
			border-radius: var(--radius-sm);
			margin-right: 0.5rem;
			transition: all 150ms var(--ease-out);
		}

		.action-btn:hover {
			color: var(--color-white);
			background: rgba(255, 255, 255, 0.05);
		}

		.action-btn--danger {
			color: #ef4444;
		}

		.action-btn--danger:hover {
			color: #fca5a5;
			background: rgba(239, 68, 68, 0.1);
		}

		.action-btn--active {
			background: rgba(15, 164, 175, 0.18);
			color: var(--color-teal-light);
		}

		.blog-admin__pagination {
			gap: 1rem;
			margin-top: 1.5rem;
		}

		.blog-admin__page-btn {
			padding: 0.55rem 1rem;
			font-size: 0.875rem;
		}

		.blog-admin__page-info {
			font-size: 0.875rem;
		}
	}

	/* ====================================================================
	   BULK ACTION BAR
	   ==================================================================== */
	.bulk-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 0.875rem;
		background: rgba(15, 164, 175, 0.08);
		border: 1px solid rgba(15, 164, 175, 0.25);
		border-radius: var(--radius-2xl);
		margin-bottom: 0.75rem;
	}

	.bulk-bar__count {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-teal-light);
		white-space: nowrap;
	}

	.bulk-bar__select {
		min-height: 2.5rem;
		padding: 0.35rem 0.6rem;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-md);
		color: var(--color-grey-200);
		font-size: 0.75rem;
		cursor: pointer;
	}

	.bulk-bar__apply {
		min-height: 2.5rem;
		padding: 0.35rem 0.875rem;
		background: rgba(15, 164, 175, 0.2);
		border: 1px solid rgba(15, 164, 175, 0.4);
		border-radius: var(--radius-md);
		color: var(--color-teal-light);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 150ms var(--ease-out);
	}

	.bulk-bar__apply:hover:not(:disabled) {
		background: rgba(15, 164, 175, 0.3);
	}

	.bulk-bar__apply:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.bulk-bar__clear {
		margin-left: auto;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		background: transparent;
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-grey-400);
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}

	.bulk-bar__clear:hover {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-white);
	}

	/* Checkbox columns */
	.th-check,
	.td-check {
		width: 2rem;
		text-align: center;
	}

	.tr--selected {
		background: rgba(15, 164, 175, 0.05) !important;
	}

	/* ====================================================================
	   QUICK EDIT
	   ==================================================================== */
	.qe-row td {
		padding: 0;
	}

	.qe-cell {
		background: rgba(15, 164, 175, 0.04);
		border-top: 1px solid rgba(15, 164, 175, 0.2);
		border-bottom: 1px solid rgba(15, 164, 175, 0.2);
	}

	.qe-form {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: 0.875rem;
		padding: 0.875rem 1rem;
	}

	.qe-form__fields {
		display: flex;
		flex-wrap: wrap;
		gap: 0.875rem;
		flex: 1;
	}

	.qe-label {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		min-width: 10rem;
		flex: 1;
	}

	.qe-label__text {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.qe-input,
	.qe-select {
		min-height: 2.25rem;
		padding: 0.45rem 0.6rem;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-md);
		color: var(--color-grey-200);
		font-size: 0.875rem;
		outline: none;
		transition:
			border-color 150ms,
			box-shadow 150ms;
	}

	.qe-input:focus,
	.qe-select:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.qe-form__actions {
		display: flex;
		gap: 0.5rem;
	}

	.qe-save {
		min-height: 2.25rem;
		padding: 0.45rem 1rem;
		background: var(--color-teal);
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-white);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 150ms var(--ease-out);
	}

	.qe-save:hover:not(:disabled) {
		background: var(--color-teal-light);
	}

	.qe-save:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.qe-cancel {
		min-height: 2.25rem;
		padding: 0.45rem 0.875rem;
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-md);
		color: var(--color-grey-400);
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}

	.qe-cancel:hover {
		color: var(--color-white);
		border-color: rgba(255, 255, 255, 0.22);
	}

	@media (prefers-reduced-motion: reduce) {
		.blog-admin__skeleton-row {
			animation: none;
		}
	}
</style>
