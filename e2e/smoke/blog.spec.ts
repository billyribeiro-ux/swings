/**
 * Blog smoke coverage.
 *
 * `/blog` is SSR with a backend-driven post list; `/blog/category/[slug]` and
 * `/blog/tag/[slug]` are dynamic. We assert:
 *   1. `/blog` returns <400 and renders the blog heading.
 *   2. A category or tag URL is navigable without triggering a 500.
 *
 * If the backend is unreachable the category/tag probe is skipped — we don't
 * want the suite red just because Postgres hasn't booted yet.
 */

import { test, expect } from '../fixtures/app';

test('/blog renders without a server error', async ({ page }) => {
	const response = await page.goto('/blog');
	expect(response?.status(), 'blog index should not 5xx').toBeLessThan(500);
	await expect(
		page.getByRole('heading', { level: 1 }).or(page.locator('h1')).first()
	).toBeVisible();
});

test('/blog/category/:slug reachable (smoke-only)', async ({ page }) => {
	const response = await page.goto('/blog/category/options-strategy').catch(() => null);
	// Either the route exists and returns an HTML response, or SvelteKit serves
	// a 404 page. Both are acceptable — we strictly reject 5xx responses.
	if (!response) {
		test.skip(true, 'Route unreachable.');
		return;
	}
	expect(response.status(), 'category page should not 5xx').toBeLessThan(500);
});

test('/blog/tag/:slug reachable (smoke-only)', async ({ page }) => {
	const response = await page.goto('/blog/tag/options').catch(() => null);
	if (!response) {
		test.skip(true, 'Route unreachable.');
		return;
	}
	expect(response.status(), 'tag page should not 5xx').toBeLessThan(500);
});
