import { getPublicApiBase } from '$lib/api/publicApiBase';
import { courses } from '$lib/data/courses';
import { SITE } from '$lib/seo/config';
import type { BlogCategory, BlogTag, BlogPostListItem, PaginatedResponse } from '$lib/api/types';
import type { RequestHandler } from './$types';

const API_BASE = getPublicApiBase();
const PAGE_SIZE = 200;

const staticPages = [
	{ path: '/', priority: '1.0', changefreq: 'weekly' },
	{ path: '/about', priority: '0.8', changefreq: 'monthly' },
	{ path: '/courses', priority: '0.9', changefreq: 'weekly' },
	{ path: '/blog', priority: '0.8', changefreq: 'weekly' },
	{ path: '/pricing', priority: '0.7', changefreq: 'monthly' },
	{ path: '/pricing/monthly', priority: '0.7', changefreq: 'monthly' },
	{ path: '/pricing/annual', priority: '0.7', changefreq: 'monthly' },
	{ path: '/terms', priority: '0.3', changefreq: 'yearly' },
	{ path: '/privacy', priority: '0.3', changefreq: 'yearly' }
];

export const GET: RequestHandler = async () => {
	const today = new Date().toISOString().split('T')[0];

	const coursePages: SitemapEntry[] = courses.map((c) => ({
		path: `/courses/${c.slug}`,
		priority: '0.8',
		changefreq: 'monthly',
		lastmod: today
	}));

	const staticEntries: SitemapEntry[] = staticPages.map((page) => ({ ...page, lastmod: today }));
	let blogPages: SitemapEntry[] = [];
	let categoryPages: SitemapEntry[] = [];
	let tagPages: SitemapEntry[] = [];

	try {
		const [postsRes, categoriesRes, tagsRes] = await Promise.all([
			fetch(`${API_BASE}/api/blog/posts?page=1&per_page=${PAGE_SIZE}`),
			fetch(`${API_BASE}/api/blog/categories`),
			fetch(`${API_BASE}/api/blog/tags`)
		]);

		if (postsRes.ok) {
			const postsPayload: PaginatedResponse<BlogPostListItem> = await postsRes.json();
			blogPages = postsPayload.data.map((post) => ({
				path: `/blog/${post.slug}`,
				priority: '0.7',
				changefreq: 'weekly',
				lastmod: (post.updated_at || post.published_at || post.created_at).split('T')[0]
			}));
		}

		if (categoriesRes.ok) {
			const categories: BlogCategory[] = await categoriesRes.json();
			categoryPages = categories.map((category) => ({
				path: `/blog/category/${category.slug}`,
				priority: '0.5',
				changefreq: 'weekly',
				lastmod: today
			}));
		}

		if (tagsRes.ok) {
			const tags: BlogTag[] = await tagsRes.json();
			tagPages = tags.map((tag) => ({
				path: `/blog/tag/${tag.slug}`,
				priority: '0.5',
				changefreq: 'weekly',
				lastmod: today
			}));
		}
	} catch (error) {
		console.warn('Sitemap dynamic URL generation failed, serving static-only sitemap:', error);
	}

	const allPages = [...staticEntries, ...coursePages, ...blogPages, ...categoryPages, ...tagPages];

	const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${allPages
	.map(
		(p) => `  <url>
    <loc>${SITE.url}${p.path}</loc>
    <lastmod>${p.lastmod}</lastmod>
    <changefreq>${p.changefreq}</changefreq>
    <priority>${p.priority}</priority>
  </url>`
	)
	.join('\n')}
</urlset>`;

	return new Response(xml, {
		headers: {
			'Content-Type': 'application/xml',
			'Cache-Control': 'max-age=3600'
		}
	});
};

interface SitemapEntry {
	path: string;
	priority: string;
	changefreq: string;
	lastmod: string;
}
