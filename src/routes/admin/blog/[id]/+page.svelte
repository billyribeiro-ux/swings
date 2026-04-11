<script lang="ts">
	import { page } from '$app/state';
	import { api } from '$lib/api/client';
	import type { BlogPostResponse, CreatePostPayload, UpdatePostPayload } from '$lib/api/types';
	import PostEditor from '$lib/components/editor/PostEditor.svelte';

	let postData: BlogPostResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const postId = $derived(page.params.id);

	$effect(() => {
		loadPost();
	});

	async function loadPost() {
		loading = true;
		error = '';
		try {
			postData = await api.get<BlogPostResponse>(`/api/admin/blog/posts/${postId}`);
		} catch (e) {
			error = 'Failed to load post';
			console.error(e);
		} finally {
			loading = false;
		}
	}

	async function updatePost(
		payload: CreatePostPayload | UpdatePostPayload
	): Promise<BlogPostResponse> {
		return api.put<BlogPostResponse>(
			`/api/admin/blog/posts/${postId}`,
			payload as UpdatePostPayload
		);
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
	<svelte:boundary>
		<PostEditor mode="edit" post={postData} onSave={updatePost} />
		{#snippet failed(err, reset)}
			<div class="editor-error">
				<p>The editor crashed unexpectedly.</p>
				<p class="editor-error__detail">{err instanceof Error ? err.message : String(err)}</p>
				<button class="editor-error__reset" onclick={reset}>Try again</button>
			</div>
		{/snippet}
	</svelte:boundary>
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
		flex-direction: column;
		gap: 0.75rem;
		text-align: center;
	}

	.editor-error__detail {
		font-size: 0.75rem;
		color: rgba(239, 68, 68, 0.7);
		max-width: 40rem;
	}

	.editor-error__reset {
		padding: 0.5rem 1.25rem;
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.35);
		border-radius: 0.4rem;
		color: #ef4444;
		cursor: pointer;
		font-size: 0.875rem;
	}
</style>
