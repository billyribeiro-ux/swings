import { readFile } from 'node:fs/promises';
import process from 'node:process';

const failures = [];

function assertRule(condition, message) {
	if (!condition) failures.push(message);
}

async function read(path) {
	return readFile(new URL(path, `file://${process.cwd()}/`), 'utf8');
}

const [
	rootLayoutSource,
	seoComponentSource,
	sitemapSource,
	robotsSource,
	pricingSource,
	termsSource,
	privacySource
] = await Promise.all([
	read('src/routes/+layout.svelte'),
	read('src/lib/seo/Seo.svelte'),
	read('src/routes/sitemap.xml/+server.ts'),
	read('static/robots.txt'),
	read('src/routes/pricing/+page.svelte'),
	read('src/routes/terms/+page.svelte'),
	read('src/routes/privacy/+page.svelte')
]);

assertRule(
	seoComponentSource.includes('new URL(url, SITE.url).toString()'),
	'Seo.svelte must normalize canonical URLs to absolute URLs.'
);

assertRule(
	!termsSource.includes('canonical="/') && !privacySource.includes('canonical="/'),
	'Terms and privacy pages must not use relative canonical props.'
);

assertRule(
	pricingSource.includes('<Seo'),
	'Pricing hub page must use shared Seo.svelte metadata component.'
);

assertRule(
	rootLayoutSource.includes('meta name="robots" content="noindex, nofollow"'),
	'Root layout must apply noindex robots directives for non-public app routes.'
);

assertRule(sitemapSource.includes("path: '/pricing'"), 'Sitemap must include /pricing.');
assertRule(
	sitemapSource.includes('/blog/category/') && sitemapSource.includes('/blog/tag/'),
	'Sitemap must include blog category and tag URL coverage.'
);

assertRule(
	robotsSource.includes('Sitemap: https://explosiveswings.com/sitemap.xml'),
	'robots.txt must reference canonical sitemap URL.'
);

for (const requiredDisallow of ['Disallow: /dashboard', 'Disallow: /admin', 'Disallow: /api/']) {
	assertRule(robotsSource.includes(requiredDisallow), `robots.txt missing "${requiredDisallow}".`);
}

if (failures.length > 0) {
	console.error('SEO audit failed with the following issues:');
	for (const failure of failures) console.error(`- ${failure}`);
	process.exit(1);
}

console.log('SEO audit passed.');
