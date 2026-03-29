import { courses } from '$lib/data/courses';
import type { RequestHandler } from './$types';

const SITE_URL = 'https://explosiveswings.com';

const staticPages = [
	{ path: '/', priority: '1.0', changefreq: 'weekly' },
	{ path: '/about', priority: '0.8', changefreq: 'monthly' },
	{ path: '/courses', priority: '0.9', changefreq: 'weekly' },
	{ path: '/blog', priority: '0.8', changefreq: 'weekly' },
	{ path: '/pricing/monthly', priority: '0.7', changefreq: 'monthly' },
	{ path: '/pricing/annual', priority: '0.7', changefreq: 'monthly' }
];

export const GET: RequestHandler = async () => {
	const today = new Date().toISOString().split('T')[0];

	const coursePages = courses.map((c) => ({
		path: `/courses/${c.slug}`,
		priority: '0.8',
		changefreq: 'monthly'
	}));

	const allPages = [...staticPages, ...coursePages];

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
