/**
 * Stable, role/label-based locator factories.
 *
 * Specs should consume these helpers instead of hand-rolling CSS selectors.
 * When a PE7 primitive (Button / Dialog / Drawer / Toast) changes its internal
 * DOM, a central locator swap fixes every test at once. Prefer `getByRole` and
 * `getByLabel` over CSS — they double as accessibility assertions.
 */

import type { Locator, Page } from '@playwright/test';

/** Marketing / public shell landmarks. */
export const landmarks = {
	primaryNav: (p: Page): Locator => p.getByRole('navigation', { name: /primary/i }),
	main: (p: Page): Locator => p.locator('main').first(),
	footer: (p: Page): Locator => p.locator('footer').first(),
	breadcrumb: (p: Page): Locator => p.getByRole('navigation', { name: /breadcrumb/i })
};

/** PE7 Button locator by accessible name (case-insensitive regex). */
export function button(p: Page, name: RegExp | string): Locator {
	return p.getByRole('button', { name });
}

/** PE7 Dialog locator — role=dialog with optional accessible name. */
export function dialog(p: Page, name?: RegExp | string): Locator {
	return p.getByRole('dialog', name ? { name } : undefined);
}

/** PE7 Drawer locator — implemented as role=dialog + aria-modal. */
export function drawer(p: Page, name?: RegExp | string): Locator {
	return p.getByRole('dialog', name ? { name } : undefined);
}

/** Toast container lives in the live-region status slot. */
export function toast(p: Page, text?: RegExp | string): Locator {
	const region = p.getByRole('status').or(p.getByRole('alert'));
	if (text === undefined) return region;
	return region.filter({ hasText: text });
}

/** Auth-form fields (login + register share label text). */
export const authForm = {
	email: (p: Page): Locator => p.getByLabel(/email/i),
	password: (p: Page): Locator => p.getByLabel(/^password$/i),
	confirmPassword: (p: Page): Locator => p.getByLabel(/confirm password/i),
	name: (p: Page): Locator => p.getByLabel(/full name/i),
	submit: (p: Page): Locator =>
		p.getByRole('button', { name: /^(sign in|create account|sign up)$/i })
};

/** Dashboard sidebar items, keyed by the public `label` string. */
export function dashboardNavLink(p: Page, label: RegExp | string): Locator {
	return p.getByRole('link', { name: label });
}
