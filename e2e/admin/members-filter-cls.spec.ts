/**
 * Mutation-CLS gate for the admin members filter tabs.
 *
 * The forensic sweep (`forensic-cls.spec.ts`) only measures CLS on initial
 * page load. The user-visible bug — "click All roles / Admins / Active /
 * Suspended and the page jumps" — is *post-click* CLS: the row count
 * changes, the table-wrap shrinks/grows, and everything below the table
 * shifts. This spec asserts that no filter click produces a layout shift
 * above Google's CLS "good" threshold (0.1) — and we're stricter still
 * (< 0.01) because a principal-grade UI shouldn't shift at all on a
 * filter swap.
 *
 * Implementation:
 *   - PerformanceObserver({ type: 'layout-shift', buffered: true })
 *     captures every layout-shift entry. We snapshot the cumulative
 *     score before and after each click and assert the delta.
 *   - We click filters via their `aria-label="Filter by …"` ancestors
 *     and the tab text — same DOM the user touches.
 *   - We `waitForResponse('/api/admin/members…')` after every click so
 *     the assertion runs after the new row set has rendered.
 */

import { authedAdminTest as test, type AdminFixtures } from '../fixtures/auth';
import type { AppFixtures } from '../fixtures/app';
import { expect } from '@playwright/test';

type Fixtures = AppFixtures & AdminFixtures;

const FILTER_TABS: Array<{ group: 'role' | 'status'; label: string }> = [
	{ group: 'role', label: 'All roles' },
	{ group: 'role', label: 'Admins' },
	{ group: 'role', label: 'Members' },
	{ group: 'status', label: 'Any status' },
	{ group: 'status', label: 'Active' },
	{ group: 'status', label: 'Suspended' },
	{ group: 'status', label: 'Banned' }
];

// Hard threshold. Google's "good CLS" is 0.1; we want effectively zero on a
// click that should be a pure data swap inside an existing list.
const MAX_DELTA_PER_CLICK = 0.01;

test.describe.configure({ mode: 'serial' });

// Both viewports the page actually renders. The mobile cards view and the
// tablet+ table view are *different DOM*, not a CSS reflow — bugs in one
// don't reproduce in the other. Test both.
const VIEWPORTS = [
	{ name: 'desktop', width: 1280, height: 800, listSelector: 'table.m-table tbody tr' },
	{ name: 'mobile', width: 390, height: 844, listSelector: '.members-page__cards .member-card' }
] as const;

for (const vp of VIEWPORTS) {
	test(`admin members filter tabs do not produce layout shift @ ${vp.name}`, async ({
		admin: _admin,
		page
	}: Fixtures) => {
		test.setTimeout(120_000);

		// Install the observer BEFORE the page loads so `buffered: true` catches
		// every shift from first paint onward. We expose helpers on `window` to
		// snapshot/diff the cumulative score from spec code.
		await page.addInitScript(() => {
			const w = window as unknown as {
				__clsTotal: number;
				__clsEntries: Array<{
					value: number;
					hadRecentInput: boolean;
					startTime: number;
					sources: Array<{
						tag: string | undefined;
						html: string;
						prev: { x: number; y: number; w: number; h: number } | null;
						curr: { x: number; y: number; w: number; h: number } | null;
					}>;
				}>;
			};
			w.__clsTotal = 0;
			w.__clsEntries = [];
			try {
				const po = new PerformanceObserver((list) => {
					for (const e of list.getEntries()) {
						const ls = e as PerformanceEntry & {
							value: number;
							hadRecentInput: boolean;
							sources?: Array<{
								node?: { nodeName?: string; outerHTML?: string };
								currentRect: DOMRectReadOnly | null;
								previousRect: DOMRectReadOnly | null;
							}>;
						};
						if (ls.hadRecentInput) continue;
						w.__clsTotal += ls.value;
						w.__clsEntries.push({
							value: ls.value,
							hadRecentInput: ls.hadRecentInput,
							startTime: ls.startTime,
							sources: (ls.sources ?? []).map((s) => ({
								tag: s.node?.nodeName,
								html: (s.node?.outerHTML ?? '').slice(0, 200),
								prev: s.previousRect
									? {
											x: s.previousRect.x,
											y: s.previousRect.y,
											w: s.previousRect.width,
											h: s.previousRect.height
										}
									: null,
								curr: s.currentRect
									? {
											x: s.currentRect.x,
											y: s.currentRect.y,
											w: s.currentRect.width,
											h: s.currentRect.height
										}
									: null
							}))
						});
					}
				});
				po.observe({ type: 'layout-shift', buffered: true });
			} catch {
				/* layout-shift not supported (firefox/webkit) — spec only meaningful in chromium */
			}
		});

		await page.setViewportSize({ width: vp.width, height: vp.height });

		await page.goto('/admin/members', { waitUntil: 'domcontentloaded' });

		// Wait for the initial list to render in whichever DOM block is visible
		// at this viewport (table on desktop, cards on mobile).
		await page.locator(vp.listSelector).first().waitFor({ timeout: 15_000 });

		// Allow any post-paint shifts (font-swap, async images, etc.) to settle so
		// they don't pollute the per-click deltas.
		await page.waitForTimeout(1500);

		type Snapshot = {
			total: number;
			entries: Array<{
				value: number;
				sources: Array<{
					tag: string | undefined;
					html: string;
					prev: { x: number; y: number; w: number; h: number } | null;
					curr: { x: number; y: number; w: number; h: number } | null;
				}>;
			}>;
		};

		async function readCls(): Promise<Snapshot> {
			return page.evaluate(() => {
				const w = window as unknown as {
					__clsTotal: number;
					__clsEntries: Array<{
						value: number;
						sources: Array<{
							tag: string | undefined;
							html: string;
							prev: { x: number; y: number; w: number; h: number } | null;
							curr: { x: number; y: number; w: number; h: number } | null;
						}>;
					}>;
				};
				return {
					total: w.__clsTotal ?? 0,
					entries: (w.__clsEntries ?? []).map((e) => ({
						value: e.value,
						sources: e.sources
					}))
				};
			});
		}

		// Load-time CLS gate — verifies the skeleton → real-data transition is
		// a pure in-place row swap. If this fails, the reserved-space invariant
		// (skeleton row-h must equal real row-h, skeleton count must equal
		// PER_PAGE) was broken by a downstream edit.
		const loadSnapshot = await readCls();

		console.log(`[${vp.name} load] cls=${loadSnapshot.total.toFixed(4)}`);
		if (loadSnapshot.total > 0.001) {
			console.log(
				`[${vp.name} load] offending sources:\n` +
					loadSnapshot.entries
						.flatMap((e) =>
							e.sources.map(
								(s) =>
									`  Δ=${e.value.toFixed(4)} ${s.tag} ${s.prev?.h ?? '?'}→${s.curr?.h ?? '?'}h ${s.prev?.y ?? '?'}→${s.curr?.y ?? '?'}y ${s.html.slice(0, 120)}`
							)
						)
						.slice(0, 6)
						.join('\n')
			);
		}
		expect(
			loadSnapshot.total,
			`[${vp.name}] Load-time CLS too high — reserved-space invariant broken.`
		).toBeLessThan(0.02);

		const failures: Array<{
			click: string;
			delta: number;
			offenders: Array<{ tag: string | undefined; html: string }>;
		}> = [];

		for (const tab of FILTER_TABS) {
			const before = await readCls();

			// Click the tab inside its labeled tablist so the role/status selectors
			// don't collide on identical labels (e.g. there's a "Members" tab AND
			// a column heading literally called "Members" elsewhere).
			const groupAria = tab.group === 'role' ? 'Filter by role' : 'Filter by status';
			const tabLocator = page
				.locator(`[role="tablist"][aria-label="${groupAria}"]`)
				.getByRole('tab', { name: tab.label, exact: true });

			// Fire the click and wait for the resulting list refetch. The page
			// only triggers a single GET /api/admin/members per filter change.
			await Promise.all([
				page.waitForResponse(
					(r) => r.url().includes('/api/admin/members') && r.request().method() === 'GET',
					{ timeout: 10_000 }
				),
				tabLocator.click()
			]);

			// Let the response render. 350ms is generous: the Svelte 5 micro-task
			// queue fires the DOM patch within one frame, but we want any post-
			// patch reflow (e.g. a tbody re-layout) to settle into the observer.
			await page.waitForTimeout(350);

			const after = await readCls();
			const delta = after.total - before.total;
			const newEntries = after.entries.slice(before.entries.length);

			console.log(
				`[${vp.name} ${tab.group}:${tab.label}] cls Δ=${delta.toFixed(4)} entries=${newEntries.length}`
			);

			if (delta > MAX_DELTA_PER_CLICK) {
				const offenders = newEntries.flatMap((e) =>
					e.sources.map((s) => ({ tag: s.tag, html: s.html }))
				);
				failures.push({ click: `${tab.group}:${tab.label}`, delta, offenders });
			}
		}

		if (failures.length > 0) {
			const summary = failures
				.map(
					(f) =>
						`  ${f.click} → Δ=${f.delta.toFixed(4)}\n` +
						f.offenders
							.slice(0, 3)
							.map((o) => `    ${o.tag}: ${o.html}`)
							.join('\n')
				)
				.join('\n');
			expect(
				failures,
				`[${vp.name}] Filter clicks produced layout shift above ${MAX_DELTA_PER_CLICK}:\n${summary}`
			).toEqual([]);
		}
	});
}
