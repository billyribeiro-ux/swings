<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicCascade, EASE, DURATION } from '$lib/utils/animations';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, articleSchema, buildJsonLd } from '$lib/seo/jsonld';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Article from 'phosphor-svelte/lib/Article';
	import CalendarBlank from 'phosphor-svelte/lib/CalendarBlank';
	import Clock from 'phosphor-svelte/lib/Clock';
	import type { BlogPostListItem, BlogCategory, PaginatedResponse } from '$lib/api/types';

	const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3001';

	let heroRef: HTMLElement | undefined = $state();
	let posts: BlogPostListItem[] = $state([]);
	let categories: BlogCategory[] = $state([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);

	onMount(() => {
		let ctx: ReturnType<typeof gsap.context> | undefined;

		if (heroRef) {
			ctx = gsap.context(() => {
				createCinematicCascade(heroRef!, [
					{
						selector: '.blog-badge',
						duration: DURATION.fast,
						ease: EASE.snappy,
						y: 20,
						blur: 6,
						scale: 0.9,
						overlap: 0
					},
					{
						selector: '.blog-title',
						duration: DURATION.cinematic,
						ease: EASE.cinematic,
						y: 36,
						blur: 10,
						scale: 0.95,
						overlap: 0.6
					},
					{
						selector: '.blog-subtitle',
						duration: DURATION.slow,
						ease: EASE.soft,
						y: 28,
						blur: 8,
						scale: 0.98,
						overlap: 0.6
					}
				]);
			}, heroRef as HTMLElement);
		}

		loadPosts();
		loadCategories();

		return () => ctx?.revert();
	});

	async function loadPosts() {
		loading = true;
		try {
			const res = await fetch(`${API_BASE}/api/blog/posts?page=${page}&per_page=12`);
			if (res.ok) {
				const data: PaginatedResponse<BlogPostListItem> = await res.json();
				posts = data.data;
				total = data.total;
				totalPages = data.total_pages;
			}
		} catch (e) {
			console.error('Failed to load blog posts', e);
		} finally {
			loading = false;
		}
	}

	async function loadCategories() {
		try {
			const res = await fetch(`${API_BASE}/api/blog/categories`);
			if (res.ok) categories = await res.json();
		} catch (e) {
			console.error('Failed to load categories', e);
		}
	}

	function nextPage() {
		if (page < totalPages) {
			page++;
			loadPosts();
		}
	}

	function prevPage() {
		if (page > 1) {
			page--;
			loadPosts();
		}
	}

	const jsonLd = $derived(
		buildJsonLd([
			webPageSchema({
				path: '/blog',
				title: 'Blog - Explosive Swings',
				description:
					'Options trading insights, strategies, and education from the Explosive Swings team.',
				speakable: '.blog-title, .blog-subtitle'
			}),
			...posts.map((p) =>
				articleSchema({
					title: p.title,
					description: p.excerpt || '',
					path: `/blog/${p.slug}`,
					datePublished: p.published_at || p.created_at
				})
			)
		])
	);
</script>

<Seo
	title="Blog - Explosive Swings"
	description="Options trading insights, strategies, and education from the Explosive Swings team."
	ogTitle="Options Trading Blog - Explosive Swings"
	{jsonLd}
/>

<!-- Hero -->
<section bind:this={heroRef} class="page-hero">
	<div class="page-hero__grid-overlay"></div>

	<div class="page-hero__inner">
		<div class="blog-badge page-badge">
			<Article size={18} weight="duotone" color="#15C5D1" />
			<span class="page-badge__text">Blog</span>
		</div>

		<h1 class="blog-title page-hero__title">Trading Insights & Education</h1>

		<p class="blog-subtitle page-hero__subtitle">
			Strategies, analysis, and lessons from the trading desk to help you level up your options
			game.
		</p>
	</div>
</section>

<!-- Category Filters -->
{#if categories.length > 0}
	<section class="page-section page-section--white" style="padding-bottom: 0;">
		<div class="page-container">
			<div class="blog-categories">
				<a href="/blog" class="blog-categories__link blog-categories__link--active">All</a>
				{#each categories as cat (cat.id)}
					<a href="/blog/category/{cat.slug}" class="blog-categories__link">{cat.name}</a>
				{/each}
			</div>
		</div>
	</section>
{/if}

<!-- Posts Grid -->
<section class="page-section page-section--off-white">
	<div class="page-container">
		{#if loading}
			<div class="blog-loading">Loading posts...</div>
		{:else if posts.length === 0}
			<div class="blog-empty">
				<Article size={48} weight="duotone" color="#0FA4AF" class="blog-coming-soon__icon" />
				<h3 class="blog-coming-soon__title">No Posts Yet</h3>
				<p class="blog-coming-soon__desc">
					We're working on new content. Check back soon for trading insights, strategies, and
					education.
				</p>
			</div>
		{:else}
			<ScrollReveal>
				<div class="blog-grid">
					{#each posts as post, i (post.id)}
						<article class="reveal-item blog-card" style="transition-delay: {i * 0.08}s">
							{#if post.featured_image_url}
								<a href="/blog/{post.slug}" class="blog-card__image-link">
									<img
										src={post.featured_image_url}
										alt={post.title}
										class="blog-card__image"
										loading="lazy"
									/>
								</a>
							{/if}
							<div class="blog-card__body">
								<div class="blog-card__meta">
									<span class="blog-card__meta-item">
										<CalendarBlank size={14} weight="bold" class="blog-card__meta-icon" />
										{new Date(post.published_at || post.created_at).toLocaleDateString('en-US', {
											month: 'short',
											day: 'numeric',
											year: 'numeric'
										})}
									</span>
									<span>•</span>
									<span class="blog-card__meta-item">
										<Clock size={14} weight="bold" class="blog-card__meta-icon" />
										{post.reading_time_minutes} min read
									</span>
								</div>

								{#if post.categories.length > 0}
									<span class="blog-card__category">{post.categories[0].name}</span>
								{/if}

								<h2 class="blog-card__title">{post.title}</h2>

								{#if post.excerpt}
									<p class="blog-card__excerpt">{post.excerpt}</p>
								{/if}

								<a href="/blog/{post.slug}" class="blog-card__link">
									Read More
									<ArrowRight size={14} weight="bold" />
								</a>
							</div>
						</article>
					{/each}
				</div>
			</ScrollReveal>

			{#if totalPages > 1}
				<div class="blog-pagination">
					<button class="blog-pagination__btn" disabled={page <= 1} onclick={prevPage}
						>← Previous</button
					>
					<span class="blog-pagination__info">Page {page} of {totalPages}</span>
					<button class="blog-pagination__btn" disabled={page >= totalPages} onclick={nextPage}
						>Next →</button
					>
				</div>
			{/if}
		{/if}
	</div>
</section>

<style>
	.page-hero {
		position: relative;
		overflow: hidden;
		padding-top: 4rem;
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.page-hero__grid-overlay {
		position: absolute;
		inset: 0;
		opacity: 0.02;
		background-image:
			linear-gradient(to right, white 1px, transparent 1px),
			linear-gradient(to bottom, white 1px, transparent 1px);
		background-size: 60px 60px;
	}

	.page-hero__inner {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 5rem 1rem;
		text-align: center;
	}

	@media (min-width: 640px) {
		.page-hero__inner {
			padding: 5rem 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-hero__inner {
			padding: 7rem 2rem;
		}
	}

	.page-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(15, 164, 175, 0.3);
		background-color: rgba(15, 164, 175, 0.1);
		padding: 0.5rem 1rem;
		margin-bottom: 1.5rem;
	}

	.page-badge__text {
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.page-hero__title {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.15;
		margin-bottom: 1.5rem;
	}

	@media (min-width: 640px) {
		.page-hero__title {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 768px) {
		.page-hero__title {
			font-size: clamp(2.5rem, 5vw, 3rem);
		}
	}
	@media (min-width: 1024px) {
		.page-hero__title {
			font-size: clamp(3rem, 5vw, 3.75rem);
		}
	}

	.page-hero__subtitle {
		color: var(--color-grey-300);
		font-size: 1rem;
		line-height: 1.65;
		max-width: 42rem;
		margin: 0 auto;
	}

	@media (min-width: 640px) {
		.page-hero__subtitle {
			font-size: var(--fs-lg);
		}
	}
	@media (min-width: 1024px) {
		.page-hero__subtitle {
			font-size: var(--fs-xl);
		}
	}

	.page-section {
		padding: 4rem 0;
	}
	@media (min-width: 640px) {
		.page-section {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.page-section {
			padding: 7rem 0;
		}
	}

	.page-section--white {
		background-color: var(--color-white);
	}
	.page-section--off-white {
		background-color: var(--color-off-white);
	}

	.page-container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.page-container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-container {
			padding: 0 2rem;
		}
	}

	/* Blog grid */
	.blog-grid {
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 640px) {
		.blog-grid {
			gap: 2rem;
		}
	}
	@media (min-width: 768px) {
		.blog-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
	@media (min-width: 1024px) {
		.blog-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.blog-card {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		background-color: var(--color-white);
		box-shadow: var(--shadow-sm);
		outline: 1px solid rgba(216, 220, 228, 0.8);
		outline-offset: -1px;
		transition: all 500ms var(--ease-out);
	}

	.blog-card:hover {
		transform: translateY(-0.25rem);
		box-shadow: var(--shadow-xl);
		outline-color: rgba(15, 164, 175, 0.3);
	}

	.blog-card__body {
		flex: 1;
		padding: 1.5rem;
	}

	@media (min-width: 640px) {
		.blog-card__body {
			padding: 2rem;
		}
	}

	.blog-card__meta {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		margin-bottom: 1rem;
	}

	.blog-card__meta-item {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}

	:global(.blog-card__meta-icon) {
		color: var(--color-grey-400) !important;
	}

	.blog-card__category {
		display: inline-block;
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
		border-radius: var(--radius-lg);
		padding: 0.25rem 0.75rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		margin-bottom: 0.75rem;
	}

	.blog-card__title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		line-height: 1.3;
		margin-bottom: 0.75rem;
		transition: color 300ms var(--ease-out);
	}

	.blog-card:hover .blog-card__title {
		color: var(--color-teal);
	}

	.blog-card__excerpt {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
		margin-bottom: 1.5rem;
	}

	.blog-card__link {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		transition: all 300ms var(--ease-out);
	}

	.blog-card:hover .blog-card__link {
		color: var(--color-teal-light);
		transform: translateX(2px);
	}

	/* Categories filter */
	.blog-categories {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		justify-content: center;
		padding: 1.5rem 0;
	}

	.blog-categories__link {
		padding: 0.4rem 1rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-grey-600);
		text-decoration: none;
		border: 1px solid var(--color-grey-200);
		transition: all 200ms var(--ease-out);
	}

	.blog-categories__link:hover {
		color: var(--color-teal);
		border-color: var(--color-teal);
	}

	.blog-categories__link--active {
		background-color: var(--color-teal);
		color: var(--color-white);
		border-color: var(--color-teal);
	}

	/* Featured image */
	.blog-card__image-link {
		display: block;
		overflow: hidden;
	}

	.blog-card__image {
		width: 100%;
		height: 12rem;
		object-fit: cover;
		transition: transform 500ms var(--ease-out);
	}

	.blog-card:hover .blog-card__image {
		transform: scale(1.04);
	}

	/* Loading / empty */
	.blog-loading {
		text-align: center;
		padding: 4rem;
		color: var(--color-grey-500);
		font-size: var(--fs-lg);
	}

	.blog-empty {
		max-width: 42rem;
		margin: 0 auto;
		border: 1px solid var(--color-grey-200);
		background-color: var(--color-off-white);
		border-radius: var(--radius-2xl);
		padding: 2.5rem;
		text-align: center;
	}

	:global(.blog-coming-soon__icon) {
		margin: 0 auto 1rem !important;
	}

	.blog-coming-soon__title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		margin-bottom: 0.75rem;
	}

	.blog-coming-soon__desc {
		color: var(--color-grey-600);
		line-height: 1.65;
	}

	/* Pagination */
	.blog-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 1.5rem;
		margin-top: 3rem;
	}

	.blog-pagination__btn {
		padding: 0.6rem 1.25rem;
		border: 1px solid var(--color-grey-200);
		border-radius: var(--radius-lg);
		background: var(--color-white);
		color: var(--color-navy);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}

	.blog-pagination__btn:hover:not(:disabled) {
		border-color: var(--color-teal);
		color: var(--color-teal);
	}

	.blog-pagination__btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.blog-pagination__info {
		font-size: var(--fs-sm);
		color: var(--color-grey-500);
	}
</style>
