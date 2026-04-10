<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import CalendarBlank from 'phosphor-svelte/lib/CalendarBlank';
	import Clock from 'phosphor-svelte/lib/Clock';
	import { getPublicApiBase } from '$lib/api/publicApiBase';
	import type { BlogPostListItem, PaginatedResponse } from '$lib/api/types';

	const API_BASE = getPublicApiBase();

	let posts: BlogPostListItem[] = $state([]);
	let total = $state(0);
	let currentPage = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	const slug = $derived($page.params.slug ?? '');
	const tagName = $derived(slug.replace(/-/g, ' '));

	onMount(async () => {
		await loadPosts();
	});

	async function loadPosts() {
		loading = true;
		try {
			const res = await fetch(
				`${API_BASE}/api/blog/posts/tag/${slug}?page=${currentPage}&per_page=12`
			);
			if (res.ok) {
				const data: PaginatedResponse<BlogPostListItem> = await res.json();
				posts = data.data;
				total = data.total;
				totalPages = data.total_pages;
			}
		} catch (e) {
			console.error('Failed to load posts', e);
		} finally {
			loading = false;
		}
	}

	const jsonLd = $derived(
		buildJsonLd([
			webPageSchema({
				path: `/blog/tag/${slug}`,
				title: `Tag: ${tagName} - Blog - Explosive Swings`,
				description: `Posts tagged with ${tagName}.`
			})
		])
	);
</script>

<Seo
	title="Tag: {tagName} - Blog - Explosive Swings"
	description="Posts tagged with {tagName}."
	{jsonLd}
/>

<section class="tag-hero">
	<div class="tag-hero__inner">
		<a href="/blog" class="tag-hero__back">
			<ArrowLeft size={16} weight="bold" />
			Back to Blog
		</a>
		<h1 class="tag-hero__title">Tag: {tagName}</h1>
		<p class="tag-hero__count">{total} post{total !== 1 ? 's' : ''}</p>
	</div>
</section>

<section class="tag-posts">
	<div class="tag-posts__container">
		{#if loading}
			<div class="tag-posts__empty">Loading...</div>
		{:else if posts.length === 0}
			<div class="tag-posts__empty">No posts with this tag yet.</div>
		{:else}
			<ScrollReveal>
				<div class="tag-grid">
					{#each posts as post, i (post.id)}
						<article class="reveal-item tag-card" style="transition-delay: {i * 0.08}s">
							{#if post.featured_image_url}
								<a href="/blog/{post.slug}" class="tag-card__img-link">
									<img src={post.featured_image_url} alt={post.title} loading="lazy" />
								</a>
							{/if}
							<div class="tag-card__body">
								<div class="tag-card__meta">
									<CalendarBlank size={14} weight="bold" />
									{new Date(post.published_at || post.created_at).toLocaleDateString('en-US', {
										month: 'short',
										day: 'numeric',
										year: 'numeric'
									})}
									<span>·</span>
									<Clock size={14} weight="bold" />
									{post.reading_time_minutes} min
								</div>
								<h2 class="tag-card__title">
									<a href="/blog/{post.slug}">{post.title}</a>
								</h2>
								{#if post.excerpt}
									<p class="tag-card__excerpt">{post.excerpt}</p>
								{/if}
								<a href="/blog/{post.slug}" class="tag-card__link">
									Read More <ArrowRight size={14} weight="bold" />
								</a>
							</div>
						</article>
					{/each}
				</div>
			</ScrollReveal>

			{#if totalPages > 1}
				<div class="tag-pagination">
					<button
						disabled={currentPage <= 1}
						onclick={() => {
							currentPage--;
							loadPosts();
						}}>← Prev</button
					>
					<span>Page {currentPage} of {totalPages}</span>
					<button
						disabled={currentPage >= totalPages}
						onclick={() => {
							currentPage++;
							loadPosts();
						}}>Next →</button
					>
				</div>
			{/if}
		{/if}
	</div>
</section>

<style>
	.tag-hero {
		background: linear-gradient(to bottom right, var(--color-navy), var(--color-navy-mid));
		padding-top: 4rem;
	}

	.tag-hero__inner {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 3rem 1.5rem 2.5rem;
	}

	.tag-hero__back {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1rem;
	}

	.tag-hero__back:hover {
		color: var(--color-teal-light);
	}

	.tag-hero__title {
		font-family: var(--font-heading);
		font-size: clamp(1.5rem, 4vw, 2.5rem);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 0.5rem;
		text-transform: capitalize;
	}

	.tag-hero__count {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.tag-posts {
		background: var(--color-off-white);
		padding: 3rem 0 4rem;
	}

	.tag-posts__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1.5rem;
	}

	.tag-posts__empty {
		text-align: center;
		padding: 4rem;
		color: var(--color-grey-500);
	}

	.tag-grid {
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 768px) {
		.tag-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (min-width: 1024px) {
		.tag-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.tag-card {
		background: var(--color-white);
		border-radius: var(--radius-2xl);
		overflow: hidden;
		box-shadow: var(--shadow-sm);
		outline: 1px solid rgba(216, 220, 228, 0.8);
		outline-offset: -1px;
		transition: all 500ms var(--ease-out);
	}

	.tag-card:hover {
		transform: translateY(-0.25rem);
		box-shadow: var(--shadow-xl);
	}

	.tag-card__img-link {
		display: block;
		overflow: hidden;
	}

	.tag-card__img-link img {
		width: 100%;
		height: 12rem;
		object-fit: cover;
		transition: transform 500ms var(--ease-out);
	}

	.tag-card:hover .tag-card__img-link img {
		transform: scale(1.04);
	}

	.tag-card__body {
		padding: 1.5rem;
	}

	.tag-card__meta {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		margin-bottom: 0.75rem;
	}

	.tag-card__title {
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin-bottom: 0.5rem;
		line-height: 1.3;
	}

	.tag-card__title a {
		color: inherit;
		text-decoration: none;
		transition: color 200ms;
	}

	.tag-card:hover .tag-card__title a {
		color: var(--color-teal);
	}

	.tag-card__excerpt {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
		margin-bottom: 1rem;
	}

	.tag-card__link {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	.tag-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1.5rem;
		margin-top: 2.5rem;
	}

	.tag-pagination button {
		padding: 0.5rem 1rem;
		border: 1px solid var(--color-grey-200);
		border-radius: var(--radius-lg);
		background: var(--color-white);
		color: var(--color-navy);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
	}

	.tag-pagination button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.tag-pagination span {
		font-size: var(--fs-sm);
		color: var(--color-grey-500);
	}
</style>
