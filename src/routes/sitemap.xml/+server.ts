import { getPublicApiBase } from '$lib/api/publicApiBase';
import { courses } from '$lib/data/courses';
import type { RequestHandler } from './$types';

const SITE_URL = 'https://explosiveswings.com';
const API_BASE = getPublicApiBase();

const staticPages = [
	{ path: '/', priority: '1.0', changefreq: 'weekly' },
	{ path: '/about', priority: '0.8', changefreq: 'monthly' },
	{ path: '/courses', priority: '0.9', changefreq: 'weekly' },
	{ path: '/blog', priority: '0.8', changefreq: 'weekly' },
	{ path: '/pricing/monthly', priority: '0.7', changefreq: 'monthly' },
	{ path: '/pricing/annual', priority: '0.7', changefreq: 'monthly' },
	{ path: '/terms', priority: '0.3', changefreq: 'yearly' },
	{ path: '/privacy', priority: '0.3', changefreq: 'yearly' }
];

export const GET: RequestHandler = async () => {
	const today = new Date().toISOString().split('T')[0];

	const coursePages = courses.map((c) => ({
		path: `/courses/${c.slug}`,
		priority: '0.8',
		changefreq: 'monthly'
	}));

	// Fetch blog post slugs for sitemap
	let blogPages: { path: string; priority: string; changefreq: string }[] = [];
	try {
		const res = await fetch(`${API_BASE}/api/blog/slugs`);
		if (res.ok) {
			const slugs: string[] = await res.json();
			blogPages = slugs.map((slug) => ({
				path: `/blog/${slug}`,
				priority: '0.7',
				changefreq: 'weekly'
			}));
		}
	} catch {
		// API unavailable at build time — skip blog pages
	}

	const allPages = [...staticPages, ...coursePages, ...blogPages];

	const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${allPages
	.map(
		(p) => `  <url>
    <loc>${SITE_URL}${p.path}</loc>
    <lastmod>${today}</lastmod>
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
