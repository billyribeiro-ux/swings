/**
 * Forensic admin CLS + render sweep.
 * Visits every admin top-level page, records:
 *   - HTTP responses to /api/admin/* (status, path)
 *   - PerformanceObserver layout-shift entries with sources
 *   - Console errors
 *   - Screenshot at three timestamps (initial, 1s, 3s) to expose
 *     post-paint shifts and async data populating.
 */

import { authedAdminTest as test, type AdminFixtures } from '../fixtures/auth';
import type { AppFixtures } from '../fixtures/app';
import { writeFile, mkdir } from 'fs/promises';
import path from 'path';

type Fixtures = AppFixtures & AdminFixtures;

const OUT = '/tmp/forensic';

const ADMIN_PAGES: Array<{ slug: string; url: string }> = [
	{ slug: 'home', url: '/admin' },
	{ slug: 'analytics', url: '/admin/analytics' },
	{ slug: 'audit', url: '/admin/audit' },
	{ slug: 'author', url: '/admin/author' },
	{ slug: 'blog', url: '/admin/blog' },
	{ slug: 'blog-new', url: '/admin/blog/new' },
	{ slug: 'consent', url: '/admin/consent' },
	{ slug: 'consent-banner', url: '/admin/consent/banner' },
	{ slug: 'coupons', url: '/admin/coupons' },
	{ slug: 'coupons-new', url: '/admin/coupons/new' },
	{ slug: 'courses', url: '/admin/courses' },
	{ slug: 'dsar', url: '/admin/dsar' },
	{ slug: 'forms', url: '/admin/forms' },
	{ slug: 'forms-new', url: '/admin/forms/new' },
	{ slug: 'members', url: '/admin/members' },
	{ slug: 'notifications', url: '/admin/notifications' },
	{ slug: 'notifications-templates', url: '/admin/notifications/templates' },
	{ slug: 'orders', url: '/admin/orders' },
	{ slug: 'outbox', url: '/admin/outbox' },
	{ slug: 'popups', url: '/admin/popups' },
	{ slug: 'products', url: '/admin/products' },
	{ slug: 'security', url: '/admin/security' },
	{ slug: 'settings', url: '/admin/settings' },
	{ slug: 'subscriptions', url: '/admin/subscriptions' },
	{ slug: 'watchlists', url: '/admin/watchlists' }
];

test.describe.configure({ mode: 'serial' });

test.describe('forensic admin sweep', () => {
	test.setTimeout(180_000);

	for (const { slug, url } of ADMIN_PAGES) {
		// `app` is destructured to spin up the fixture (Playwright won't
		// initialise a fixture that isn't requested) but the spec body only
		// touches `page`. Renaming to `_app` matches the eslint allowed-
		// unused-args pattern (`/^_/u`).
		test(`forensic ${slug}`, async ({ app: _app, page }: Fixtures) => {
			await mkdir(OUT, { recursive: true });
			const consoleErrors: string[] = [];
			const consoleWarnings: string[] = [];
			const apiResponses: Array<{
				method: string;
				url: string;
				status: number;
				timing: number;
			}> = [];
			const start = Date.now();

			page.on('console', (msg) => {
				if (msg.type() === 'error') consoleErrors.push(msg.text());
				if (msg.type() === 'warning') consoleWarnings.push(msg.text());
			});
			page.on('response', (resp) => {
				const u = resp.url();
				if (u.includes('/api/')) {
					apiResponses.push({
						method: resp.request().method(),
						url: u.replace(/^https?:\/\/[^/]+/, ''),
						status: resp.status(),
						timing: Date.now() - start
					});
				}
			});

			// Set up CLS observer BEFORE navigation
			await page.addInitScript(() => {
				(window as unknown as { __clsEntries: unknown[] }).__clsEntries = [];
				try {
					const po = new PerformanceObserver((list) => {
						for (const e of list.getEntries()) {
							const ls = e as PerformanceEntry & {
								value: number;
								hadRecentInput: boolean;
								sources?: Array<{
									node?: { nodeName?: string; outerHTML?: string };
									currentRect: DOMRectReadOnly;
									previousRect: DOMRectReadOnly;
								}>;
							};
							(window as unknown as { __clsEntries: unknown[] }).__clsEntries.push({
								value: ls.value,
								hadRecentInput: ls.hadRecentInput,
								startTime: ls.startTime,
								sources: (ls.sources ?? []).map((s) => ({
									tag: s.node?.nodeName,
									html: (s.node?.outerHTML ?? '').slice(0, 220),
									prev: s.previousRect && {
										x: s.previousRect.x,
										y: s.previousRect.y,
										w: s.previousRect.width,
										h: s.previousRect.height
									},
									curr: s.currentRect && {
										x: s.currentRect.x,
										y: s.currentRect.y,
										w: s.currentRect.width,
										h: s.currentRect.height
									}
								}))
							});
						}
					});
					po.observe({ type: 'layout-shift', buffered: true });
				} catch {
					/* ignore */
				}
			});

			// Drive a real UI login first to wire up the session cookie
			// and the auth store identically to a real session.
			await page.goto('/admin/login', { waitUntil: 'domcontentloaded' });
			try {
				await page
					.locator('input[type="email"]')
					.first()
					.fill(process.env.ADMIN_EMAIL ?? '');
				await page
					.locator('input[type="password"]')
					.first()
					.fill(process.env.ADMIN_PASSWORD ?? '');
				await Promise.all([
					page.waitForResponse((r) => r.url().includes('/api/auth/login'), {
						timeout: 10_000
					}),
					page.locator('button[type="submit"]').first().click()
				]);
				await page.waitForURL((u) => !u.toString().includes('/admin/login'), {
					timeout: 10_000
				});
			} catch (e) {
				consoleErrors.push(`LOGIN_FAIL ${(e as Error).message}`);
			}

			let navStatus = 0;
			try {
				const resp = await page.goto(url, { waitUntil: 'domcontentloaded' });
				navStatus = resp?.status() ?? 0;
			} catch (e) {
				consoleErrors.push(`NAV_FAIL ${(e as Error).message}`);
			}

			// Capture three screenshots over 3.5s while async data loads.
			await page.screenshot({
				path: path.join(OUT, `shot-${slug}-0s.png`),
				fullPage: false
			});
			await page.waitForTimeout(1000);
			await page.screenshot({
				path: path.join(OUT, `shot-${slug}-1s.png`),
				fullPage: false
			});
			await page.waitForTimeout(2500);
			await page.screenshot({
				path: path.join(OUT, `shot-${slug}-3s.png`),
				fullPage: false
			});

			const cls = await page.evaluate(() => {
				const entries =
					(
						window as unknown as {
							__clsEntries: Array<{
								value: number;
								hadRecentInput: boolean;
								sources?: unknown[];
							}>;
						}
					).__clsEntries ?? [];
				const total = entries
					.filter((e) => !e.hadRecentInput)
					.reduce((s, e) => s + e.value, 0);
				return { total, count: entries.length, entries };
			});

			const evidence = {
				slug,
				url,
				navStatus,
				cls,
				consoleErrors,
				consoleWarnings,
				apiResponses
			};
			await writeFile(
				path.join(OUT, `cls-${slug}.json`),
				JSON.stringify(evidence, null, 2),
				'utf8'
			);

			// Don't fail — this is a forensic sweep, not a gate.
			console.log(
				`[forensic ${slug}] nav=${navStatus} cls=${cls.total.toFixed(4)} api=${apiResponses.length} errors=${consoleErrors.length}`
			);
		});
	}
});
