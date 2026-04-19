export interface AdminNavItem {
	href: string;
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
	{ href: '/admin/consent/policies', label: 'Policies' },
	{ href: '/admin/consent/log', label: 'Log + integrity' }
];
