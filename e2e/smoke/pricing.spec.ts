/**
 * Pricing page smoke coverage.
 *
 * Pricing lives at `/pricing`, with pinned billing-cycle variants at
 * `/pricing/monthly` and `/pricing/annual`. All three are prerendered but the
 * plan cards rely on `/api/pricing/plans` — when the backend isn't reachable
 * we still want the page shell + CTAs to render, so the test only asserts on
 * statically-rendered chrome.
 *
 * Each route has its own h1 copy (compare `/pricing`'s "Choose the Plan…" vs
 * `/pricing/monthly`'s "Weekly Watchlists…"), so the assertion pairs the
 * route with an expected regex instead of using a single shared pattern.
 */

import { test, expect } from '../fixtures/app';

const ROUTES: ReadonlyArray<{ path: string; heading: RegExp }> = [
	{ path: '/pricing', heading: /choose the plan|trading workflow/i },
	{ path: '/pricing/monthly', heading: /month-to-month|weekly watchlists/i },
	{ path: '/pricing/annual', heading: /annual plan|save\s+\d/i }
];

for (const { path, heading } of ROUTES) {
	test(`renders ${path}`, async ({ page }) => {
		const response = await page.goto(path);
		expect(response?.status(), `HTTP status for ${path}`).toBeLessThan(400);
		await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
	});
}

test('pricing CTAs lead to /register or /login', async ({ page }) => {
	await page.goto('/pricing');
	const cta = page
		.getByRole('link', { name: /get started|start now|sign up|subscribe/i })
		.first();
	// We don't hard-fail when no CTA is visible — the plans list is
	// backend-driven, and under preview without seeded plans the cards don't
	// render at all. In that case the hero copy is still exercised above.
	if (!(await cta.isVisible().catch(() => false))) {
		test.skip(true, 'No pricing CTA visible (plans not seeded).');
		return;
	}
	await cta.click();
	await expect(page).toHaveURL(/\/(login|register)/);
});
