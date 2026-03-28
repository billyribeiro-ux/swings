export interface AuthResponse {
	user: UserResponse;
	access_token: string;
	refresh_token: string;
}

export interface UserResponse {
	id: string;
	email: string;
	name: string;
	role: 'member' | 'admin';
	avatar_url: string | null;
	created_at: string;
}

export interface Subscription {
	id: string;
	user_id: string;
	stripe_customer_id: string;
	stripe_subscription_id: string;
	plan: 'monthly' | 'annual';
	status: 'active' | 'canceled' | 'past_due' | 'trialing' | 'unpaid';
	current_period_start: string;
	current_period_end: string;
	created_at: string;
	updated_at: string;
}

export interface SubscriptionResponse {
	subscription: Subscription | null;
	is_active: boolean;
}

export interface Watchlist {
	id: string;
	title: string;
	week_of: string;
	video_url: string | null;
	notes: string | null;
	published: boolean;
	published_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface WatchlistAlert {
	id: string;
	watchlist_id: string;
	ticker: string;
	direction: 'bullish' | 'bearish';
	entry_zone: string;
	invalidation: string;
	profit_zones: string[];
	notes: string | null;
	chart_url: string | null;
	created_at: string;
}

export interface WatchlistWithAlerts extends Watchlist {
	alerts: WatchlistAlert[];
}

export interface CourseEnrollment {
	id: string;
	user_id: string;
	course_id: string;
	progress: number;
	enrolled_at: string;
	completed_at: string | null;
}

export interface AdminStats {
	total_members: number;
	active_subscriptions: number;
	monthly_subscriptions: number;
	annual_subscriptions: number;
	total_watchlists: number;
	total_enrollments: number;
	recent_members: UserResponse[];
}

export interface PaginatedResponse<T> {
	data: T[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}
