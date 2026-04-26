import type { resolve } from '$app/paths';

/**
 * `resolve()` from `$app/paths` accepts only literal `RouteId` values, so we
 * tighten `href` to that exact string union. This keeps the data file
 * JSON-serialisable (plain strings — no `resolve()` calls baked in) while
 * giving consumers a typed input they can pass straight into `resolve(item.href)`.
 */
export type AdminNavHref = Parameters<typeof resolve>[0];

export interface AdminNavItem {
	href: AdminNavHref;
	label: string;
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
