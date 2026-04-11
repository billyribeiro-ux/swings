<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import CalendarBlank from 'phosphor-svelte/lib/CalendarBlank';
	import Clock from 'phosphor-svelte/lib/Clock';
	import { getPublicApiBase } from '$lib/api/publicApiBase';
	import type { BlogPostListItem, BlogCategory, PaginatedResponse } from '$lib/api/types';

	const API_BASE = getPublicApiBase();

	let posts: BlogPostListItem[] = $state([]);
	let categories: BlogCategory[] = $state([]);
	let total = $state(0);
	let currentPage = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	// `[slug]` is a required dynamic segment so SvelteKit guarantees a value at
	// runtime, but the generated `RouteParams` type widens to `string | undefined`.
	// `!` documents the runtime invariant.
	const slug = $derived(page.params.slug!);
	const categoryName = $derived(
		categories.find((c) => c.slug === slug)?.name || slug.replace(/-/g, ' ')
	);

	onMount(async () => {
		await Promise.all([loadPosts(), loadCategories()]);
	});

	async function loadPosts() {
		loading = true;
		try {
			const res = await fetch(
				`${API_BASE}/api/blog/posts/category/${slug}?page=${currentPage}&per_page=12`
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

	async function loadCategories() {
		try {
			const res = await fetch(`${API_BASE}/api/blog/categories`);
			if (res.ok) categories = await res.json();
		} catch (e) {
			console.error(e);
		}
	}

	const jsonLd = $derived(
		buildJsonLd([
			webPageSchema({
				path: `/blog/category/${slug}`,
				title: `${categoryName} - Blog - Explosive Swings`,
				description: `Posts in the ${categoryName} category.`
			})
		])
	);
</script>

<Seo
	title="{categoryName} - Blog - Explosive Swings"
	description="Posts in the {categoryName} category."
	{jsonLd}
/>

<section class="cat-hero">
	<div class="cat-hero__inner">
		<a href="/blog" class="cat-hero__back">
			<ArrowLeft size={16} weight="bold" />
			Back to Blog
		</a>
		<h1 class="cat-hero__title">Category: {categoryName}</h1>
		<p class="cat-hero__count">{total} post{total !== 1 ? 's' : ''}</p>
	</div>
</section>

<!-- Category nav -->
{#if categories.length > 0}
	<section class="cat-nav-section">
		<div class="cat-nav">
			<a href="/blog" class="cat-nav__link">All</a>
			{#each categories as cat (cat.id)}
				<a
					href="/blog/category/{cat.slug}"
					class="cat-nav__link"
					class:cat-nav__link--active={cat.slug === slug}
				>
					{cat.name}
				</a>
			{/each}
		</div>
	</section>
{/if}

<section class="cat-posts">
	<div class="cat-posts__container">
		{#if loading}
			<div class="cat-posts__empty">Loading...</div>
		{:else if posts.length === 0}
			<div class="cat-posts__empty">No posts in this category yet.</div>
		{:else}
			<ScrollReveal>
				<div class="cat-grid">
					{#each posts as post, i (post.id)}
						<article class="reveal-item cat-card" style="transition-delay: {i * 0.08}s">
							{#if post.featured_image_url}
								<a href="/blog/{post.slug}" class="cat-card__img-link">
									<img src={post.featured_image_url} alt={post.title} loading="lazy" />
								</a>
							{/if}
							<div class="cat-card__body">
								<div class="cat-card__meta">
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
								<h2 class="cat-card__title">
									<a href="/blog/{post.slug}">{post.title}</a>
								</h2>
								{#if post.excerpt}
									<p class="cat-card__excerpt">{post.excerpt}</p>
								{/if}
								<a href="/blog/{post.slug}" class="cat-card__link">
									Read More <ArrowRight size={14} weight="bold" />
								</a>
							</div>
						</article>
					{/each}
				</div>
			</ScrollReveal>

			{#if totalPages > 1}
				<div class="cat-pagination">
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
	.cat-hero {
		background: linear-gradient(to bottom right, var(--color-navy), var(--color-navy-mid));
		padding-top: 4rem;
	}

	.cat-hero__inner {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 3rem 1.5rem 2.5rem;
	}

	.cat-hero__back {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1rem;
	}

	.cat-hero__back:hover {
		color: var(--color-teal-light);
	}

	.cat-hero__title {
		font-family: var(--font-heading);
		font-size: clamp(1.5rem, 4vw, 2.5rem);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 0.5rem;
		text-transform: capitalize;
	}

	.cat-hero__count {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	/* Category nav */
	.cat-nav-section {
		background: var(--color-white);
		border-bottom: 1px solid var(--color-grey-200);
	}

	.cat-nav {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 1rem 1.5rem;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.cat-nav__link {
		padding: 0.35rem 0.85rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-grey-600);
		text-decoration: none;
		border: 1px solid var(--color-grey-200);
		transition: all 200ms;
	}

	.cat-nav__link:hover {
		color: var(--color-teal);
		border-color: var(--color-teal);
	}

	.cat-nav__link--active {
		background: var(--color-teal);
		color: var(--color-white);
		border-color: var(--color-teal);
	}

	/* Posts */
	.cat-posts {
		background: var(--color-off-white);
		padding: 3rem 0 4rem;
	}

	.cat-posts__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1.5rem;
	}

	.cat-posts__empty {
		text-align: center;
		padding: 4rem;
		color: var(--color-grey-500);
	}

	.cat-grid {
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 768px) {
		.cat-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (min-width: 1024px) {
		.cat-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.cat-card {
		background: var(--color-white);
		border-radius: var(--radius-2xl);
		overflow: hidden;
		box-shadow: var(--shadow-sm);
		outline: 1px solid rgba(216, 220, 228, 0.8);
		outline-offset: -1px;
		transition: all 500ms var(--ease-out);
	}

	.cat-card:hover {
		transform: translateY(-0.25rem);
		box-shadow: var(--shadow-xl);
	}

	.cat-card__img-link {
		display: block;
		overflow: hidden;
	}

	.cat-card__img-link img {
		width: 100%;
		height: 12rem;
		object-fit: cover;
		transition: transform 500ms var(--ease-out);
	}

	.cat-card:hover .cat-card__img-link img {
		transform: scale(1.04);
	}

	.cat-card__body {
		padding: 1.5rem;
	}

	.cat-card__meta {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		margin-bottom: 0.75rem;
	}

	.cat-card__title {
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin-bottom: 0.5rem;
		line-height: 1.3;
	}

	.cat-card__title a {
		color: inherit;
		text-decoration: none;
		transition: color 200ms;
	}

	.cat-card:hover .cat-card__title a {
		color: var(--color-teal);
	}

	.cat-card__excerpt {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
		margin-bottom: 1rem;
	}

	.cat-card__link {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	/* Pagination */
	.cat-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1.5rem;
		margin-top: 2.5rem;
	}

	.cat-pagination button {
		padding: 0.5rem 1rem;
		border: 1px solid var(--color-grey-200);
		border-radius: var(--radius-lg);
		background: var(--color-white);
		color: var(--color-navy);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
	}

	.cat-pagination button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.cat-pagination span {
		font-size: var(--fs-sm);
		color: var(--color-grey-500);
	}
</style>
