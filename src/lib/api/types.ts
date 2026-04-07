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
	bio: string | null;
	position: string | null;
	website_url: string | null;
	twitter_url: string | null;
	linkedin_url: string | null;
	youtube_url: string | null;
	instagram_url: string | null;
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

// ── Blog ───────────────────────────────────────────────────────────────

export type PostStatus =
	| 'draft'
	| 'pending_review'
	| 'published'
	| 'private'
	| 'scheduled'
	| 'trash';

export interface BlogPostResponse {
	id: string;
	author_id: string;
	author_name: string;
	author_avatar: string | null;
	author_position: string | null;
	author_bio: string | null;
	author_website: string | null;
	author_twitter: string | null;
	author_linkedin: string | null;
	author_youtube: string | null;
	title: string;
	slug: string;
	content: string;
	content_json: Record<string, unknown> | null;
	excerpt: string | null;
	featured_image_url: string | null;
	status: PostStatus;
	visibility: string;
	is_sticky: boolean;
	allow_comments: boolean;
	meta_title: string | null;
	meta_description: string | null;
	canonical_url: string | null;
	og_image_url: string | null;
	reading_time_minutes: number;
	word_count: number;
	categories: BlogCategory[];
	tags: BlogTag[];
	scheduled_at: string | null;
	published_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface BlogPostListItem {
	id: string;
	author_id: string;
	author_name: string;
	title: string;
	slug: string;
	excerpt: string | null;
	featured_image_url: string | null;
	status: PostStatus;
	is_sticky: boolean;
	reading_time_minutes: number;
	word_count: number;
	published_at: string | null;
	created_at: string;
	updated_at: string;
	categories: BlogCategory[];
	tags: BlogTag[];
}

export interface CreatePostPayload {
	title: string;
	slug?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	excerpt?: string;
	featured_image_id?: string;
	status?: PostStatus;
	visibility?: string;
	is_sticky?: boolean;
	allow_comments?: boolean;
	meta_title?: string;
	meta_description?: string;
	canonical_url?: string;
	og_image_url?: string;
	category_ids?: string[];
	tag_ids?: string[];
	scheduled_at?: string;
}

export interface UpdatePostPayload {
	title?: string;
	slug?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	excerpt?: string;
	featured_image_id?: string;
	status?: PostStatus;
	visibility?: string;
	is_sticky?: boolean;
	allow_comments?: boolean;
	meta_title?: string;
	meta_description?: string;
	canonical_url?: string;
	og_image_url?: string;
	category_ids?: string[];
	tag_ids?: string[];
	scheduled_at?: string;
}

export interface AutosavePayload {
	title?: string;
	content?: string;
	content_json?: Record<string, unknown>;
}

export interface BlogCategory {
	id: string;
	name: string;
	slug: string;
	description: string | null;
	parent_id: string | null;
	sort_order: number;
	created_at: string;
}

export interface BlogTag {
	id: string;
	name: string;
	slug: string;
	created_at: string;
}

export interface BlogRevision {
	id: string;
	post_id: string;
	author_id: string;
	author_name: string;
	title: string;
	revision_number: number;
	created_at: string;
}

export interface MediaItem {
	id: string;
	uploader_id: string;
	filename: string;
	original_filename: string;
	title: string | null;
	mime_type: string;
	file_size: number;
	width: number | null;
	height: number | null;
	alt_text: string | null;
	caption: string | null;
	storage_path: string;
	url: string;
	created_at: string;
}

export interface PostListFilters {
	page?: number;
	per_page?: number;
	status?: PostStatus;
	author_id?: string;
	category_slug?: string;
	tag_slug?: string;
	search?: string;
}
