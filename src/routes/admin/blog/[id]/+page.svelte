<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api/client';
	import type { BlogPostResponse, UpdatePostPayload } from '$lib/api/types';
	import PostEditor from '$lib/components/editor/PostEditor.svelte';

	let postData: BlogPostResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const postId = $derived($page.params.id);

	$effect(() => {
		loadPost();
	});

	async function loadPost() {
		loading = true;
		error = '';
		try {
			postData = await api.get<BlogPostResponse>(`/admin/blog/posts/${postId}`);
		} catch (e) {
			error = 'Failed to load post';
			console.error(e);
		} finally {
			loading = false;
		}
	}

	async function updatePost(payload: UpdatePostPayload): Promise<BlogPostResponse> {
		return api.put<BlogPostResponse>(`/admin/blog/posts/${postId}`, payload);
	}
</script>

<svelte:head>
	<title>{postData ? `Edit: ${postData.title}` : 'Edit Post'} — Admin</title>
</svelte:head>

{#if loading}
	<div class="editor-loading">Loading post...</div>
{:else if error}
	<div class="editor-error">{error}</div>
{:else if postData}
	<PostEditor
		mode="edit"
		post={postData}
		onSave={updatePost}
	/>
{/if}

<style>
	.editor-loading,
	.editor-error {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 50vh;
		color: var(--color-grey-400, #64748b);
		font-size: 1rem;
	}
	.editor-error {
		color: #ef4444;
	}
</style>
