import { resolve } from '$app/paths';
import type { ResolvedPathname, RouteId } from '$app/types';

/**
 * Static (parameterless) `RouteId` — i.e. routes whose path matches their id.
 * Excludes parametric routes like `/admin/coupons/[id]`, since this nav data
 * file only ever links to top-level admin sub-surfaces, not to specific rows.
 */
export type AdminNavHref = RouteId;

export interface AdminNavItem {
	href: AdminNavHref;
	label: string;
}

/**
 * Apply the SvelteKit 2 `resolve()` helper to a nav item's `href`.
 *
 * Why this wrapper exists:
 * 1. `resolve()` is *overloaded per-route* (one signature per `RouteId`), so
 *    calling it with a generic `RouteId` union won't typecheck. We widen the
 *    call signature to `(RouteId) => ResolvedPathname` exactly once, here.
 * 2. The `svelte/no-navigation-without-resolve` ESLint rule treats a return
 *    typed as `ResolvedPathname` (from `$app/types`) as already-resolved at
 *    consumer sites, so `<a href={resolveAdminHref(item.href)}>` lints clean
 *    without a second `resolve()` wrapper.
 *
 * Runtime is unchanged: SvelteKit's `resolve(...)` accepts any `RouteId` at
 * runtime (the per-route overloads are purely a TypeScript narrowing helper
 * for parametric routes; static `RouteId`s take no `params` argument).
 */
const resolveStatic = resolve as (route: RouteId) => ResolvedPathname;
export function resolveAdminHref(href: AdminNavHref): ResolvedPathname {
	return resolveStatic(href);
}

export const publicAdminRoutes = ['/admin/forgot-password', '/admin/reset-password'];

export const blogAdminItems: AdminNavItem[] = [
	{ href: '/admin/blog', label: 'All Posts' },
	{ href: '/admin/blog/new', label: 'New Post' },
	{ href: '/admin/blog/categories', label: 'Categories' },
	{ href: '/admin/blog/tags', label: 'Tags' },
	{ href: '/admin/blog/media', label: 'Media' }
];

export const courseAdminItems: AdminNavItem[] = [
	{ href: '/admin/courses', label: 'All Courses' },
	{ href: '/admin/courses/new', label: 'New Course' }
];

export const subscriptionAdminItems: AdminNavItem[] = [
	{ href: '/admin/subscriptions', label: 'Overview' },
	{ href: '/admin/subscriptions/plans', label: 'Pricing Plans' },
	{ href: '/admin/subscriptions/manual', label: 'Manual ops (comp / extend)' }
];

export const couponAdminItems: AdminNavItem[] = [
	{ href: '/admin/coupons', label: 'All Coupons' },
	{ href: '/admin/coupons/new', label: 'Create Coupon' }
];

export const popupAdminItems: AdminNavItem[] = [
	{ href: '/admin/popups', label: 'All Popups' },
	{ href: '/admin/popups/new', label: 'Create Popup' }
];

export const consentAdminItems: AdminNavItem[] = [
	{ href: '/admin/consent', label: 'Overview' },
	{ href: '/admin/consent/banner', label: 'Banners' },
	{ href: '/admin/consent/categories', label: 'Categories' },
	{ href: '/admin/consent/services', label: 'Services' },
	{ href: '/admin/consent/policy', label: 'Policy' },
	{ href: '/admin/consent/log', label: 'Log' },
	{ href: '/admin/consent/integrity', label: 'Integrity' }
];

export const notificationAdminItems: AdminNavItem[] = [
	{ href: '/admin/notifications', label: 'Overview' },
	{ href: '/admin/notifications/templates', label: 'Templates' },
	{ href: '/admin/notifications/deliveries', label: 'Deliveries' },
	{ href: '/admin/notifications/suppression', label: 'Suppression' }
];
