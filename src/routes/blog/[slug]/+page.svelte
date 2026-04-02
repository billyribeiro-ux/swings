<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { articleSchema, buildJsonLd } from '$lib/seo/jsonld';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import CalendarBlank from 'phosphor-svelte/lib/CalendarBlank';
	import Clock from 'phosphor-svelte/lib/Clock';
	import User from 'phosphor-svelte/lib/User';
	import type { BlogPostResponse } from '$lib/api/types';

	const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3001';

	let post: BlogPostResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	onMount(async () => {
		await loadPost();
	});

	async function loadPost() {
		loading = true;
		error = '';
		try {
			const res = await fetch(`${API_BASE}/api/blog/posts/${slug}`);
			if (res.ok) {
				post = await res.json();
			} else if (res.status === 404) {
				error = 'Post not found';
			} else {
				error = 'Failed to load post';
			}
		} catch (e) {
			error = 'Failed to load post';
			console.error(e);
		} finally {
			loading = false;
		}
	}

	function buildPostJsonLd(p: BlogPostResponse): string {
		return buildJsonLd([
			articleSchema({
				title: p.title,
				description: p.meta_description || p.excerpt || '',
				path: `/blog/${p.slug}`,
				datePublished: p.published_at || p.created_at,
				dateModified: p.updated_at,
				image: p.featured_image_url || undefined,
				authorName: p.author_name
			})
		]);
	}

	const jsonLd = $derived(post ? buildPostJsonLd(post) : '');
</script>

{#if post}
	<Seo
		title="{post.meta_title || post.title} - Explosive Swings"
		description={post.meta_description || post.excerpt || ''}
		ogTitle={post.meta_title || post.title}
		ogImage={post.og_image_url || post.featured_image_url || undefined}
		canonical={post.canonical_url || undefined}
		{jsonLd}
	/>
{/if}

{#if loading}
	<div class="post-loading">
		<div class="post-loading__spinner">Loading...</div>
	</div>
{:else if error}
	<div class="post-error">
		<h1>{error}</h1>
		<a href="/blog">← Back to blog</a>
	</div>
{:else if post}
	<!-- Hero -->
	<article class="post-article">
		<header class="post-header">
			<div class="post-header__inner">
				<a href="/blog" class="post-header__back">
					<ArrowLeft size={16} weight="bold" />
					Back to Blog
				</a>

				<div class="post-header__meta">
					{#each post.categories as cat}
						<a href="/blog/category/{cat.slug}" class="post-header__category">{cat.name}</a>
					{/each}
				</div>

				<h1 class="post-header__title">{post.title}</h1>

				{#if post.excerpt}
					<p class="post-header__excerpt">{post.excerpt}</p>
				{/if}

				<div class="post-header__info">
					<span class="post-header__info-item">
						<User size={16} weight="bold" />
						{post.author_name}
					</span>
					<span class="post-header__info-item">
						<CalendarBlank size={16} weight="bold" />
						{new Date(post.published_at || post.created_at).toLocaleDateString('en-US', {
							month: 'long',
							day: 'numeric',
							year: 'numeric'
						})}
					</span>
					<span class="post-header__info-item">
						<Clock size={16} weight="bold" />
						{post.reading_time_minutes} min read
					</span>
				</div>
			</div>
		</header>

		{#if post.featured_image_url}
			<div class="post-featured">
				<img src={post.featured_image_url} alt={post.title} />
			</div>
		{/if}

		<div class="post-body">
			<div class="post-content">
				{@html post.content}
			</div>

			{#if post.tags.length > 0}
				<div class="post-tags">
					<span class="post-tags__label">Tags:</span>
					{#each post.tags as tag}
						<a href="/blog/tag/{tag.slug}" class="post-tags__pill">{tag.name}</a>
					{/each}
				</div>
			{/if}

			<div class="post-footer">
				<a href="/blog" class="post-footer__link">
					<ArrowLeft size={16} weight="bold" />
					Back to all posts
				</a>
			</div>
		</div>
	</article>
{/if}

<style>
	.post-loading,
	.post-error {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 60vh;
		gap: 1rem;
		text-align: center;
	}

	.post-error h1 {
		font-size: var(--fs-2xl);
		color: var(--color-navy);
	}

	.post-error a {
		color: var(--color-teal);
		text-decoration: none;
		font-weight: var(--w-semibold);
	}

	.post-loading__spinner {
		color: var(--color-grey-500);
		font-size: var(--fs-lg);
	}

	/* Header */
	.post-header {
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
		padding-top: 4rem;
	}

	.post-header__inner {
		max-width: 50rem;
		margin: 0 auto;
		padding: 4rem 1.5rem 3rem;
	}

	@media (min-width: 1024px) {
		.post-header__inner {
			padding: 5rem 2rem 4rem;
		}
	}

	.post-header__back {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}

	.post-header__back:hover {
		color: var(--color-teal-light);
	}

	.post-header__meta {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.post-header__category {
		display: inline-block;
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
		border-radius: var(--radius-full);
		padding: 0.3rem 0.8rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: background 200ms;
	}

	.post-header__category:hover {
		background-color: rgba(15, 164, 175, 0.25);
	}

	.post-header__title {
		font-family: var(--font-heading);
		font-size: clamp(1.75rem, 4vw, 2.75rem);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.2;
		margin-bottom: 1rem;
	}

	.post-header__excerpt {
		color: var(--color-grey-300);
		font-size: var(--fs-lg);
		line-height: 1.65;
		max-width: 40rem;
		margin-bottom: 1.5rem;
	}

	.post-header__info {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		flex-wrap: wrap;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.post-header__info-item {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
	}

	/* Featured image */
	.post-featured {
		max-width: 50rem;
		margin: 0 auto;
		padding: 0 1.5rem;
		transform: translateY(-2rem);
	}

	.post-featured img {
		width: 100%;
		height: auto;
		border-radius: var(--radius-2xl);
		box-shadow: var(--shadow-xl);
	}

	/* Body */
	.post-body {
		max-width: 50rem;
		margin: 0 auto;
		padding: 2rem 1.5rem 4rem;
	}

	@media (min-width: 1024px) {
		.post-body {
			padding: 2rem 2rem 5rem;
		}
	}

	/* Article prose styles */
	.post-content {
		color: var(--color-grey-700);
		font-size: 1.05rem;
		line-height: 1.8;
	}

	.post-content :global(h1) {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin: 2.5rem 0 1rem;
	}

	.post-content :global(h2) {
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin: 2rem 0 0.75rem;
	}

	.post-content :global(h3) {
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin: 1.5rem 0 0.5rem;
	}

	.post-content :global(h4),
	.post-content :global(h5),
	.post-content :global(h6) {
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin: 1.25rem 0 0.5rem;
	}

	.post-content :global(p) {
		margin: 0 0 1.25rem;
	}

	.post-content :global(a) {
		color: var(--color-teal);
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	.post-content :global(a:hover) {
		color: var(--color-teal-light);
	}

	.post-content :global(blockquote) {
		border-left: 4px solid var(--color-teal);
		padding: 0.75rem 1.25rem;
		margin: 1.5rem 0;
		background: var(--color-off-white);
		border-radius: 0 var(--radius-lg) var(--radius-lg) 0;
		color: var(--color-grey-600);
		font-style: italic;
	}

	.post-content :global(ul),
	.post-content :global(ol) {
		padding-left: 1.5rem;
		margin: 1rem 0;
	}

	.post-content :global(li) {
		margin: 0.35rem 0;
	}

	.post-content :global(img) {
		max-width: 100%;
		height: auto;
		border-radius: var(--radius-xl);
		margin: 1.5rem 0;
	}

	.post-content :global(pre) {
		background: var(--color-navy);
		color: var(--color-grey-200);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
		overflow-x: auto;
		margin: 1.5rem 0;
		font-size: 0.9rem;
		line-height: 1.6;
	}

	.post-content :global(code) {
		background: rgba(15, 164, 175, 0.08);
		padding: 0.15rem 0.4rem;
		border-radius: var(--radius-md);
		font-size: 0.9em;
		color: var(--color-teal);
	}

	.post-content :global(pre code) {
		background: none;
		padding: 0;
		color: inherit;
	}

	.post-content :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin: 1.5rem 0;
	}

	.post-content :global(th),
	.post-content :global(td) {
		border: 1px solid var(--color-grey-200);
		padding: 0.6rem 0.75rem;
		text-align: left;
	}

	.post-content :global(th) {
		background: var(--color-off-white);
		font-weight: var(--w-semibold);
		color: var(--color-navy);
	}

	.post-content :global(hr) {
		border: none;
		border-top: 1px solid var(--color-grey-200);
		margin: 2rem 0;
	}

	.post-content :global(mark) {
		background: rgba(234, 179, 8, 0.2);
		padding: 0.1rem 0.2rem;
		border-radius: 0.15rem;
	}

	/* Tags */
	.post-tags {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-top: 2.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--color-grey-200);
	}

	.post-tags__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-grey-500);
	}

	.post-tags__pill {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border: 1px solid var(--color-grey-200);
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-600);
		text-decoration: none;
		transition: all 200ms var(--ease-out);
	}

	.post-tags__pill:hover {
		color: var(--color-teal);
		border-color: var(--color-teal);
	}

	/* Footer nav */
	.post-footer {
		margin-top: 2rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--color-grey-200);
	}

	.post-footer__link {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
		transition: color 200ms;
	}

	.post-footer__link:hover {
		color: var(--color-teal-light);
	}
</style>
