<script lang="ts">
	import { api } from '$lib/api/client';
	import type { BlogPostListItem, PaginatedResponse, PostStatus } from '$lib/api/types';

	let posts: BlogPostListItem[] = $state([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let statusFilter: PostStatus | '' = $state('');
	let search = $state('');
	let searchTimeout: ReturnType<typeof setTimeout>;

	$effect(() => {
		loadPosts();
	});

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
			console.error('Failed to load posts', e);
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
		if (!confirm('Move this post to trash?')) return;
		try {
			await api.put(`/api/admin/blog/posts/${id}/status`, { status: 'trash' });
			loadPosts();
		} catch (e) {
			console.error('Failed to trash post', e);
		}
	}

	async function hardDelete(id: string) {
		if (!confirm('Permanently delete this post? This cannot be undone.')) return;
		try {
			await api.delete(`/api/admin/blog/posts/${id}`);
			loadPosts();
		} catch (e) {
			console.error('Failed to delete post', e);
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
</script>

<svelte:head>
	<title>Blog Posts — Admin</title>
</svelte:head>

<div class="blog-admin">
	<div class="blog-admin__header">
		<h1>Blog Posts</h1>
		<a href="/admin/blog/new" class="btn-primary">+ New Post</a>
	</div>

	<!-- Filters -->
	<div class="blog-admin__filters">
		<div class="blog-admin__status-tabs">
			<button class:active={statusFilter === ''} onclick={() => changeStatus('')}>All</button>
			<button class:active={statusFilter === 'published'} onclick={() => changeStatus('published')}
				>Published</button
			>
			<button class:active={statusFilter === 'draft'} onclick={() => changeStatus('draft')}
				>Drafts</button
			>
			<button
				class:active={statusFilter === 'pending_review'}
				onclick={() => changeStatus('pending_review')}>Pending</button
			>
			<button class:active={statusFilter === 'scheduled'} onclick={() => changeStatus('scheduled')}
				>Scheduled</button
			>
			<button class:active={statusFilter === 'trash'} onclick={() => changeStatus('trash')}
				>Trash</button
			>
		</div>

		<input
			id="post-search"
			name="search"
			type="search"
			class="blog-admin__search"
			placeholder="Search posts..."
			oninput={handleSearch}
		/>
	</div>

	<!-- Posts table -->
	{#if loading}
		<div class="blog-admin__loading">Loading...</div>
	{:else if posts.length === 0}
		<div class="blog-admin__empty">No posts found.</div>
	{:else}
		<!-- Mobile: Card view -->
		<div class="blog-admin__cards">
			{#each posts as post (post.id)}
				<div class="post-card">
					<div class="post-card__header">
						<a href="/admin/blog/{post.id}" class="post-card__title">{post.title}</a>
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
								{#each post.categories as cat}
									<span class="cat-pill">{cat.name}</span>
								{/each}
							</span>
						</div>
					{/if}
					<div class="post-card__meta">
						{post.word_count} words · {post.reading_time_minutes} min read
					</div>
					<div class="post-card__actions">
						<a href="/admin/blog/{post.id}" class="post-card__btn post-card__btn--edit">Edit</a>
						{#if post.status === 'trash'}
							<button
								class="post-card__btn post-card__btn--delete"
								onclick={() => hardDelete(post.id)}>Delete</button
							>
						{:else}
							<button
								class="post-card__btn post-card__btn--delete"
								onclick={() => deletePost(post.id)}>Trash</button
							>
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
						<th>Title</th>
						<th>Author</th>
						<th>Categories</th>
						<th>Status</th>
						<th>Date</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each posts as post (post.id)}
						<tr>
							<td>
								<a href="/admin/blog/{post.id}" class="post-title-link">
									{post.title}
								</a>
								{#if post.is_sticky}<span class="sticky-badge">Sticky</span>{/if}
								<div class="post-meta-row">
									{post.word_count} words · {post.reading_time_minutes} min read
								</div>
							</td>
							<td class="td-author">{post.author_name}</td>
							<td class="td-cats">
								{#each post.categories as cat}
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
								<a href="/admin/blog/{post.id}" class="action-link">Edit</a>
								{#if post.status === 'trash'}
									<button class="action-btn action-btn--danger" onclick={() => hardDelete(post.id)}
										>Delete</button
									>
								{:else}
									<button class="action-btn action-btn--danger" onclick={() => deletePost(post.id)}
										>Trash</button
									>
								{/if}
							</td>
						</tr>
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
					}}>← Prev</button
				>
				<span>Page {page} of {totalPages} ({total} posts)</span>
				<button
					disabled={page >= totalPages}
					onclick={() => {
						page++;
						loadPosts();
					}}>Next →</button
				>
			</div>
		{/if}
	{/if}
</div>

<style>
	/* Mobile-first base styles */
	.blog-admin {
		max-width: 100%;
	}

	.blog-admin__header {
		margin-bottom: 1rem;
	}

	.blog-admin__header h1 {
		font-size: var(--fs-xl, 1.25rem);
		font-weight: var(--w-bold, 700);
		color: var(--color-white, #fff);
		margin: 0 0 0.75rem 0;
	}

	.btn-primary {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.55rem 1rem;
		border-radius: var(--radius-lg, 0.5rem);
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-weight: var(--w-semibold, 600);
		font-size: var(--fs-xs, 0.75rem);
		text-decoration: none;
		border: none;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.btn-primary:hover {
		opacity: 0.9;
	}

	.blog-admin__filters {
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.blog-admin__status-tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}

	.blog-admin__status-tabs button {
		padding: 0.35rem 0.6rem;
		border: none;
		border-radius: var(--radius-md, 0.25rem);
		background: transparent;
		color: var(--color-grey-400, #64748b);
		font-size: var(--fs-xs, 0.7rem);
		cursor: pointer;
	}

	.blog-admin__status-tabs button:hover {
		color: #fff;
	}

	.blog-admin__status-tabs button.active {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light, #15c5d1);
	}

	.blog-admin__search {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-lg, 0.375rem);
		background: rgba(0, 0, 0, 0.2);
		color: #fff;
		font-size: var(--fs-sm, 0.8rem);
		outline: none;
	}

	.blog-admin__search:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.blog-admin__loading,
	.blog-admin__empty {
		text-align: center;
		padding: 2rem;
		color: var(--color-grey-400, #64748b);
	}

	/* Mobile: Card view */
	.blog-admin__cards {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.blog-admin__table-wrap {
		display: none;
	}

	.post-card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background-color: var(--color-navy-mid, #0f172a);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-lg, 0.5rem);
	}

	.post-card__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.post-card__title {
		color: var(--color-white, #fff);
		font-weight: var(--w-semibold, 600);
		font-size: var(--fs-base, 0.9rem);
		text-decoration: none;
		flex: 1;
	}

	.post-card__title:hover {
		color: var(--color-teal-light, #15c5d1);
	}

	.post-card__row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.post-card__label {
		font-size: var(--fs-xs, 0.7rem);
		color: var(--color-grey-400, #64748b);
	}

	.post-card__value {
		font-size: var(--fs-sm, 0.8rem);
		color: var(--color-grey-300, #cbd5e1);
		text-align: right;
	}

	.post-card__cats {
		display: flex;
		flex-wrap: wrap;
		gap: 0.15rem;
		justify-content: flex-end;
	}

	.post-card__meta {
		font-size: var(--fs-xs, 0.7rem);
		color: var(--color-grey-400, #64748b);
	}

	.post-card__actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
		padding-top: 0.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.post-card__btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius-lg, 0.5rem);
		font-size: var(--fs-xs, 0.75rem);
		font-weight: var(--w-medium, 500);
		text-decoration: none;
		cursor: pointer;
		transition: background-color 200ms var(--ease-out, ease-out);
	}

	.post-card__btn--edit {
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal, #0fa4af);
		border: none;
	}

	.post-card__btn--edit:hover {
		background-color: rgba(15, 164, 175, 0.25);
	}

	.post-card__btn--delete {
		background-color: rgba(239, 68, 68, 0.08);
		color: #ef4444;
		border: none;
	}

	.post-card__btn--delete:hover {
		background-color: rgba(239, 68, 68, 0.2);
	}

	.sticky-badge--card {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-md, 0.2rem);
		background: rgba(234, 179, 8, 0.15);
		color: #eab308;
		font-size: var(--fs-xs, 0.65rem);
		font-weight: var(--w-semibold, 600);
		margin-top: 0.25rem;
	}

	.cat-pill {
		display: inline-block;
		padding: 0.15rem 0.4rem;
		border-radius: var(--radius-md, 0.2rem);
		background: rgba(255, 255, 255, 0.06);
		font-size: var(--fs-xs, 0.65rem);
	}

	.badge {
		display: inline-block;
		padding: 0.2rem 0.5rem;
		border-radius: var(--radius-md, 0.25rem);
		font-size: var(--fs-xs, 0.65rem);
		font-weight: var(--w-semibold, 600);
		white-space: nowrap;
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

	.blog-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.blog-admin__pagination button {
		padding: 0.4rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-md, 0.25rem);
		background: transparent;
		color: var(--color-grey-300, #94a3b8);
		font-size: var(--fs-xs, 0.75rem);
		cursor: pointer;
	}

	.blog-admin__pagination button:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.blog-admin__pagination span {
		font-size: var(--fs-xs, 0.75rem);
		color: var(--color-grey-400, #64748b);
	}

	/* Tablet+: Show table, hide cards */
	@media (min-width: 768px) {
		.blog-admin__header {
			display: flex;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 1.5rem;
		}

		.blog-admin__header h1 {
			font-size: var(--fs-2xl, 1.5rem);
			margin: 0;
		}

		.btn-primary {
			padding: 0.6rem 1.25rem;
			font-size: var(--fs-sm, 0.875rem);
		}

		.blog-admin__filters {
			flex-direction: row;
			gap: 1rem;
			margin-bottom: 1rem;
		}

		.blog-admin__status-tabs button {
			padding: 0.35rem 0.75rem;
			font-size: var(--fs-xs, 0.8rem);
		}

		.blog-admin__search {
			width: 14rem;
		}

		.blog-admin__cards {
			display: none;
		}

		.blog-admin__table-wrap {
			display: block;
			overflow-x: auto;
		}

		.blog-admin__table {
			width: 100%;
			border-collapse: collapse;
		}

		.blog-admin__table th {
			text-align: left;
			padding: 0.6rem 0.75rem;
			font-size: var(--fs-xs, 0.75rem);
			font-weight: var(--w-semibold, 600);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			color: var(--color-grey-400, #64748b);
			border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		}

		.blog-admin__table td {
			padding: 0.75rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			font-size: var(--fs-sm, 0.875rem);
			color: var(--color-grey-200, #e2e8f0);
			vertical-align: top;
		}

		.post-title-link {
			color: #fff;
			font-weight: var(--w-semibold, 600);
			text-decoration: none;
		}

		.post-title-link:hover {
			color: var(--color-teal-light, #15c5d1);
		}

		.post-meta-row {
			font-size: var(--fs-xs, 0.75rem);
			color: var(--color-grey-400, #64748b);
			margin-top: 0.15rem;
		}

		.sticky-badge {
			display: inline-block;
			padding: 0.1rem 0.4rem;
			border-radius: var(--radius-md, 0.2rem);
			background: rgba(234, 179, 8, 0.15);
			color: #eab308;
			font-size: var(--fs-xs, 0.65rem);
			font-weight: var(--w-semibold, 600);
			margin-left: 0.5rem;
			vertical-align: middle;
		}

		.td-author {
			white-space: nowrap;
		}

		.td-date {
			white-space: nowrap;
			font-size: var(--fs-xs, 0.8rem);
		}

		.td-actions {
			white-space: nowrap;
		}

		.action-link {
			color: var(--color-teal-light, #15c5d1);
			text-decoration: none;
			font-size: var(--fs-xs, 0.8rem);
			margin-right: 0.75rem;
		}

		.action-link:hover {
			text-decoration: underline;
		}

		.action-btn {
			border: none;
			background: none;
			font-size: var(--fs-xs, 0.8rem);
			cursor: pointer;
			padding: 0;
		}

		.action-btn--danger {
			color: #ef4444;
		}

		.action-btn--danger:hover {
			text-decoration: underline;
		}

		.blog-admin__pagination {
			gap: 1rem;
			margin-top: 1.5rem;
		}

		.blog-admin__pagination button {
			font-size: var(--fs-xs, 0.8rem);
		}

		.blog-admin__pagination span {
			font-size: var(--fs-xs, 0.8rem);
		}
	}
</style>
