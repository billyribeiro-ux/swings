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
			let url = `/admin/blog/posts?page=${page}&per_page=20`;
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
			await api.put(`/admin/blog/posts/${id}/status`, { status: 'trash' });
			loadPosts();
		} catch (e) {
			console.error('Failed to trash post', e);
		}
	}

	async function hardDelete(id: string) {
		if (!confirm('Permanently delete this post? This cannot be undone.')) return;
		try {
			await api.delete(`/admin/blog/posts/${id}`);
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
			<button class:active={statusFilter === 'published'} onclick={() => changeStatus('published')}>Published</button>
			<button class:active={statusFilter === 'draft'} onclick={() => changeStatus('draft')}>Drafts</button>
			<button class:active={statusFilter === 'pending_review'} onclick={() => changeStatus('pending_review')}>Pending</button>
			<button class:active={statusFilter === 'scheduled'} onclick={() => changeStatus('scheduled')}>Scheduled</button>
			<button class:active={statusFilter === 'trash'} onclick={() => changeStatus('trash')}>Trash</button>
		</div>

		<input
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
									<button class="action-btn action-btn--danger" onclick={() => hardDelete(post.id)}>Delete</button>
								{:else}
									<button class="action-btn action-btn--danger" onclick={() => deletePost(post.id)}>Trash</button>
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
				<button disabled={page <= 1} onclick={() => { page--; loadPosts(); }}>← Prev</button>
				<span>Page {page} of {totalPages} ({total} posts)</span>
				<button disabled={page >= totalPages} onclick={() => { page++; loadPosts(); }}>Next →</button>
			</div>
		{/if}
	{/if}
</div>

<style>
	.blog-admin {
		max-width: 100%;
	}

	.blog-admin__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1.5rem;
	}

	.blog-admin__header h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: #fff;
		margin: 0;
	}

	.btn-primary {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.6rem 1.25rem;
		border-radius: 0.5rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-weight: 600;
		font-size: 0.875rem;
		text-decoration: none;
		border: none;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.btn-primary:hover {
		opacity: 0.9;
	}

	.blog-admin__filters {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.blog-admin__status-tabs {
		display: flex;
		gap: 0.25rem;
	}

	.blog-admin__status-tabs button {
		padding: 0.35rem 0.75rem;
		border: none;
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-grey-400, #64748b);
		font-size: 0.8rem;
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
		padding: 0.4rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.375rem;
		background: rgba(0, 0, 0, 0.2);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
		width: 14rem;
	}

	.blog-admin__search:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.blog-admin__loading,
	.blog-admin__empty {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400, #64748b);
	}

	.blog-admin__table-wrap {
		overflow-x: auto;
	}

	.blog-admin__table {
		width: 100%;
		border-collapse: collapse;
	}

	.blog-admin__table th {
		text-align: left;
		padding: 0.6rem 0.75rem;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-400, #64748b);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.blog-admin__table td {
		padding: 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		font-size: 0.875rem;
		color: var(--color-grey-200, #e2e8f0);
		vertical-align: top;
	}

	.post-title-link {
		color: #fff;
		font-weight: 600;
		text-decoration: none;
	}

	.post-title-link:hover {
		color: var(--color-teal-light, #15c5d1);
	}

	.post-meta-row {
		font-size: 0.75rem;
		color: var(--color-grey-400, #64748b);
		margin-top: 0.15rem;
	}

	.sticky-badge {
		display: inline-block;
		padding: 0.1rem 0.4rem;
		border-radius: 0.2rem;
		background: rgba(234, 179, 8, 0.15);
		color: #eab308;
		font-size: 0.65rem;
		font-weight: 600;
		margin-left: 0.5rem;
		vertical-align: middle;
	}

	.td-author {
		white-space: nowrap;
	}

	.td-date {
		white-space: nowrap;
		font-size: 0.8rem;
	}

	.cat-pill {
		display: inline-block;
		padding: 0.1rem 0.4rem;
		border-radius: 0.2rem;
		background: rgba(255, 255, 255, 0.06);
		font-size: 0.7rem;
		margin: 0.1rem 0.15rem;
	}

	.badge {
		display: inline-block;
		padding: 0.2rem 0.5rem;
		border-radius: 0.25rem;
		font-size: 0.7rem;
		font-weight: 600;
	}

	.badge--draft { background: rgba(148, 163, 184, 0.15); color: #94a3b8; }
	.badge--pending { background: rgba(234, 179, 8, 0.15); color: #eab308; }
	.badge--published { background: rgba(34, 197, 94, 0.15); color: #22c55e; }
	.badge--private { background: rgba(139, 92, 246, 0.15); color: #8b5cf6; }
	.badge--scheduled { background: rgba(59, 130, 246, 0.15); color: #3b82f6; }
	.badge--trash { background: rgba(239, 68, 68, 0.15); color: #ef4444; }

	.td-actions {
		white-space: nowrap;
	}

	.action-link {
		color: var(--color-teal-light, #15c5d1);
		text-decoration: none;
		font-size: 0.8rem;
		margin-right: 0.75rem;
	}

	.action-link:hover {
		text-decoration: underline;
	}

	.action-btn {
		border: none;
		background: none;
		font-size: 0.8rem;
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
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		margin-top: 1.5rem;
	}

	.blog-admin__pagination button {
		padding: 0.4rem 0.75rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-grey-300, #94a3b8);
		font-size: 0.8rem;
		cursor: pointer;
	}

	.blog-admin__pagination button:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.blog-admin__pagination span {
		font-size: 0.8rem;
		color: var(--color-grey-400, #64748b);
	}
</style>
