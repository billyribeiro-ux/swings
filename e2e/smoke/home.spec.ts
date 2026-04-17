/**
 * Home page smoke coverage — cheap, static, runs in every browser engine.
 *
 * What we check:
 *   1. The hero <h1> renders.
 *   2. The primary navigation landmark is present.
 *   3. The footer is present.
 *   4. The page title is non-empty and the <html lang="…"> attribute is set.
 *   5. No uncaught console errors during the initial render (tolerating noisy
 *      analytics 404s we can't fix from frontend).
 *   6. An optional axe-core pass — only runs when `@axe-core/playwright` is
 *      present in the dev dependencies.
 */

import { test, expect } from '../fixtures/app';
import { HomePage } from '../pages/HomePage';
import { landmarks } from '../fixtures/selectors';

test.describe('home page', () => {
	test('renders hero, nav, and footer without console errors', async ({ page, app }) => {
		const errors: string[] = [];
		page.on('console', (msg) => {
			if (msg.type() === 'error') errors.push(msg.text());
		});
		page.on('pageerror', (err) => errors.push(err.message));

		const home = new HomePage(page);
		await home.open();
		void app; // `app` intentionally unused; imported so the fixture chain evaluates.

		await expect(home.hero()).toBeVisible();
		await expect(landmarks.primaryNav(page)).toBeVisible();
		await expect(landmarks.footer(page)).toBeVisible();

		// Only flag genuinely app-level errors. We tolerate network-level 404s
		// bubbling from analytics beacons in local preview.
		const appErrors = errors.filter(
			(line) => !/Failed to load resource|ERR_CONNECTION|favicon|\/api\/analytics/i.test(line)
		);
		expect(appErrors, `unexpected console errors: ${JSON.stringify(appErrors)}`).toEqual([]);
	});

	test('document metadata is populated', async ({ page }) => {
		await page.goto('/');
		await expect(page).toHaveTitle(/.+/);
		const lang = await page.locator('html').getAttribute('lang');
		expect(lang, 'html[lang] must be set for a11y').toBeTruthy();
	});

	test('axe-core a11y (gated on optional dep)', async ({ page }) => {
		await page.goto('/');
		// Dynamic import so the suite doesn't hard-require the optional dep.
		let hasAxe = false;
		let builder: unknown = null;
		try {
			const mod = await import('@axe-core/playwright').catch(() => null);
			if (mod && 'default' in mod) {
				builder = new (mod as unknown as { default: new (p: typeof page) => unknown }).default(page);
				hasAxe = true;
			}
		} catch {
			hasAxe = false;
		}
		test.skip(!hasAxe, 'Install @axe-core/playwright (devDep) to enable accessibility gating.');
		const analyze = (builder as { analyze: () => Promise<{ violations: unknown[] }> }).analyze;
		const results = await analyze.call(builder);
		expect(results.violations, `axe violations: ${JSON.stringify(results.violations, null, 2)}`).toEqual([]);
	});
});
