<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import type { CreatePostPayload, BlogPostResponse } from '$lib/api/types';
	import PostEditor from '$lib/components/editor/PostEditor.svelte';

	async function createPost(payload: CreatePostPayload): Promise<BlogPostResponse> {
		return api.post<BlogPostResponse>('/api/admin/blog/posts', payload);
	}

	async function handleSave(post: BlogPostResponse) {
		await goto(`/admin/blog/${post.id}`);
	}
</script>

<svelte:head>
	<title>New Post — Admin</title>
</svelte:head>

<PostEditor mode="create" onSave={createPost} onSaved={handleSave} />
