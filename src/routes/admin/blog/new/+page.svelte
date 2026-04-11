<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import type { CreatePostPayload, UpdatePostPayload, BlogPostResponse } from '$lib/api/types';
	import PostEditor from '$lib/components/editor/PostEditor.svelte';

	async function createPost(
		payload: CreatePostPayload | UpdatePostPayload
	): Promise<BlogPostResponse> {
		return api.post<BlogPostResponse>('/api/admin/blog/posts', payload);
	}

	async function handleSave(post: BlogPostResponse) {
		await goto(`/admin/blog/${post.id}`);
	}
</script>

<svelte:head>
	<title>New Post — Admin</title>
</svelte:head>

<svelte:boundary>
	<PostEditor mode="create" onSave={createPost} onSaved={handleSave} />
	{#snippet failed(err, reset)}
		<div class="editor-crash">
			<p>The editor crashed unexpectedly.</p>
			<p class="editor-crash__detail">{err instanceof Error ? err.message : String(err)}</p>
			<button onclick={reset}>Try again</button>
		</div>
	{/snippet}
</svelte:boundary>

<style>
	.editor-crash {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		min-height: 50vh;
		color: #ef4444;
		text-align: center;
	}
	.editor-crash__detail {
		font-size: 0.75rem;
		opacity: 0.7;
	}
	.editor-crash button {
		padding: 0.5rem 1.25rem;
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.35);
		border-radius: 0.4rem;
		color: #ef4444;
		cursor: pointer;
	}
</style>
