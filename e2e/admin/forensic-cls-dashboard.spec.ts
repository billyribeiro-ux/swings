/**
 * Forensic member-dashboard CLS sweep — mirror of `forensic-cls.spec.ts` but
 * scoped to `/dashboard/*`, the member-facing self-service surface that has
 * no CLS coverage today. Same `PerformanceObserver({ type: 'layout-shift' })`
 * shape so the JSON outputs are directly comparable.
 *
 * Outputs `/tmp/forensic/cls-dash-<slug>.json` with `{ total, count, entries }`
 * where each entry includes `sources[].html` (first 220 chars) so the offending
 * DOM node is identifiable from the evidence alone — no second pass needed.
 */

import { authedMemberTest as test, type MemberFixtures } from '../fixtures/auth';
import type { AppFixtures } from '../fixtures/app';
import { writeFile, mkdir } from 'fs/promises';
import path from 'path';

type Fixtures = AppFixtures & MemberFixtures;

const OUT = '/tmp/forensic';

const DASHBOARD_PAGES: Array<{ slug: string; url: string }> = [
	{ slug: 'home', url: '/dashboard' },
	{ slug: 'watchlists', url: '/dashboard/watchlists' },
	{ slug: 'courses', url: '/dashboard/courses' },
	{ slug: 'account', url: '/dashboard/account' },
	{ slug: 'account-details', url: '/dashboard/account/details' },
	{ slug: 'account-orders', url: '/dashboard/account/orders' },
	{ slug: 'account-subscriptions', url: '/dashboard/account/subscriptions' },
	{ slug: 'account-coupons', url: '/dashboard/account/coupons' },
	{ slug: 'account-billing-address', url: '/dashboard/account/billing-address' },
	{ slug: 'account-payment-methods', url: '/dashboard/account/payment-methods' }
];

test.describe.configure({ mode: 'serial' });

test.describe('forensic dashboard sweep', () => {
	test.setTimeout(180_000);

	for (const { slug, url } of DASHBOARD_PAGES) {
		test(`forensic-dash ${slug}`, async ({ member: _member, page }: Fixtures) => {
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

			// Set up CLS observer BEFORE navigation so `buffered: true` catches the
			// shifts that fire during the first paint (the worst CLS offenders are
			// almost always pre-DOMContentLoaded — skeleton → real content).
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

			let navStatus = 0;
			try {
				const resp = await page.goto(url, { waitUntil: 'domcontentloaded' });
				navStatus = resp?.status() ?? 0;
			} catch (e) {
				consoleErrors.push(`NAV_FAIL ${(e as Error).message}`);
			}

			// Three captures — initial, +1s, +3s — to expose post-paint shifts as
			// async data populates. Matches the admin sweep cadence so JSON shapes
			// are directly comparable.
			await page.screenshot({
				path: path.join(OUT, `shot-dash-${slug}-0s.png`),
				fullPage: false
			});
			await page.waitForTimeout(1000);
			await page.screenshot({
				path: path.join(OUT, `shot-dash-${slug}-1s.png`),
				fullPage: false
			});
			await page.waitForTimeout(2500);
			await page.screenshot({
				path: path.join(OUT, `shot-dash-${slug}-3s.png`),
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
				path.join(OUT, `cls-dash-${slug}.json`),
				JSON.stringify(evidence, null, 2),
				'utf8'
			);

			// Forensic sweep, not a gate — log the totals; don't fail the spec.
			console.log(
				`[forensic-dash ${slug}] nav=${navStatus} cls=${cls.total.toFixed(4)} api=${apiResponses.length} errors=${consoleErrors.length}`
			);
		});
	}
});
