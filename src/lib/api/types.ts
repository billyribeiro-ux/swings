/**
 * @deprecated FDN-02 — this file is the legacy hand-written DTO mirror. Prefer
 * importing from `./schema.d.ts` (generated from the committed OpenAPI snapshot
 * by `scripts/openapi-to-ts.mjs`). Kept alive because many consumers still
 * import from here; new code should use the generated types.
 */

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
	// ADM-15 lifecycle + billing profile fields. All optional; absent means
	// the account is not in that state / never collected the column.
	suspended_at?: string | null;
	suspended_until?: string | null;
	suspension_reason?: string | null;
	banned_at?: string | null;
	ban_reason?: string | null;
	email_verified_at?: string | null;
	billing_line1?: string | null;
	billing_line2?: string | null;
	billing_city?: string | null;
	billing_state?: string | null;
	billing_postal_code?: string | null;
	billing_country?: string | null;
	phone?: string | null;
}

// ── ADM-15 admin members lifecycle DTOs ────────────────────────────────

export interface BillingAddress {
	line1?: string | null | undefined;
	line2?: string | null | undefined;
	city?: string | null | undefined;
	state?: string | null | undefined;
	postal_code?: string | null | undefined;
	country?: string | null | undefined;
}

export interface UpdateMemberRequest {
	name?: string | undefined;
	email?: string | undefined;
	phone?: string | undefined;
	billing_address?: BillingAddress | undefined;
}

export interface SuspendMemberRequest {
	reason?: string;
	until?: string;
}

export interface MemberActivityEntry {
	action: string;
	actor_id: string;
	actor_role: string;
	created_at: string;
	metadata: Record<string, unknown>;
}

export interface MemberPaymentFailure {
	stripe_invoice_id: string | null;
	amount_cents: number | null;
	currency: string | null;
	failure_code: string | null;
	failure_message: string | null;
	attempt_count: number;
	created_at: string;
}

export interface MemberDetailResponse {
	user: UserResponse;
	subscription: Subscription | null;
	activity: MemberActivityEntry[];
	payment_failures: MemberPaymentFailure[];
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
	/** Present when Stripe subscription metadata includes `swings_pricing_plan_id`. */
	pricing_plan_id?: string | null;
	created_at: string;
	updated_at: string;
}

export interface SubscriptionStatusResponse {
	subscription: Subscription | null;
	is_active: boolean;
}

export interface BillingPortalResponse {
	url: string;
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

export type DashboardRange =
	| 'last_7_days'
	| 'last_30_days'
	| 'last_90_days'
	| 'year_to_date'
	| 'custom';

export interface PeriodWindow {
	new_members: number;
	new_subscriptions: number;
	canceled_subscriptions: number;
	new_enrollments: number;
	new_watchlists: number;
	revenue_cents: number;
}

export interface AdminStats {
	// Lifetime totals (unchanged behavior — "as of now")
	total_members: number;
	active_subscriptions: number;
	monthly_subscriptions: number;
	annual_subscriptions: number;
	total_watchlists: number;
	total_enrollments: number;
	recent_members: UserResponse[];
	// Range-scoped fields
	range: DashboardRange;
	from: string;
	to: string;
	period: PeriodWindow;
	previous_period: PeriodWindow;
}

export interface AnalyticsTimeBucket {
	date: string;
	page_views: number;
	unique_sessions: number;
	impressions: number;
}

export interface AnalyticsTopPage {
	path: string;
	views: number;
	sessions: number;
}

export interface AnalyticsRecentSale {
	id: string;
	event_type: string;
	amount_cents: number;
	user_email: string;
	created_at: string;
}

export interface AnalyticsCtrPoint {
	date: string;
	cta_id: string;
	impressions: number;
	clicks: number;
	ctr: number;
}

export interface AnalyticsSummary {
	from: string;
	to: string;
	total_page_views: number;
	total_sessions: number;
	total_impressions: number;
	bounced_sessions: number;
	bounce_eligible_sessions: number;
	bounce_rate: number;
	mrr_cents: number;
	arr_cents: number;
	active_subscribers: number;
	period_revenue_cents: number;
	time_series: AnalyticsTimeBucket[];
	top_pages: AnalyticsTopPage[];
	ctr_series: AnalyticsCtrPoint[];
	recent_sales: AnalyticsRecentSale[];
}

export interface AdminRevenueResponse {
	data: { date: string; revenue_cents: number }[];
}

export interface PaginatedResponse<T> {
	data: T[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

// ── Blog ───────────────────────────────────────────────────────────────

export interface PostMeta {
	id: string;
	post_id: string;
	meta_key: string;
	meta_value: string;
	created_at: string;
	updated_at: string;
}

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
	/** Status before the post was moved to trash (used when restoring). */
	pre_trash_status?: PostStatus | null;
	trashed_at?: string | null;
	visibility: string;
	is_password_protected: boolean;
	format: string;
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
	meta: PostMeta[];
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
	slug?: string | undefined;
	content?: string | undefined;
	content_json?: Record<string, unknown> | undefined;
	excerpt?: string | undefined;
	featured_image_id?: string | undefined;
	status?: PostStatus | undefined;
	visibility?: string | undefined;
	is_sticky?: boolean | undefined;
	allow_comments?: boolean | undefined;
	meta_title?: string | undefined;
	meta_description?: string | undefined;
	canonical_url?: string | undefined;
	og_image_url?: string | undefined;
	category_ids?: string[] | undefined;
	tag_ids?: string[] | undefined;
	scheduled_at?: string | undefined;
	post_password?: string | undefined;
	author_id?: string | undefined;
	format?: string | undefined;
}

export interface UpdatePostPayload {
	title?: string | undefined;
	slug?: string | undefined;
	content?: string | undefined;
	content_json?: Record<string, unknown> | undefined;
	excerpt?: string | undefined;
	featured_image_id?: string | undefined;
	status?: PostStatus | undefined;
	visibility?: string | undefined;
	is_sticky?: boolean | undefined;
	allow_comments?: boolean | undefined;
	meta_title?: string | undefined;
	meta_description?: string | undefined;
	canonical_url?: string | undefined;
	og_image_url?: string | undefined;
	category_ids?: string[] | undefined;
	tag_ids?: string[] | undefined;
	scheduled_at?: string | undefined;
	post_password?: string | undefined;
	author_id?: string | undefined;
	format?: string | undefined;
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
	focal_x: number;
	focal_y: number;
	created_at: string;
}

// ── Courses ───────────────────────────────────────────────────────────

export interface Course {
	id: string;
	title: string;
	slug: string;
	description: string;
	short_description: string | null;
	thumbnail_url: string | null;
	trailer_video_url: string | null;
	difficulty: 'beginner' | 'intermediate' | 'advanced';
	instructor_id: string;
	price_cents: number;
	currency: string;
	is_free: boolean;
	is_included_in_subscription: boolean;
	sort_order: number;
	published: boolean;
	published_at: string | null;
	estimated_duration_minutes: number;
	created_at: string;
	updated_at: string;
}

export interface CourseModule {
	id: string;
	course_id: string;
	title: string;
	description: string | null;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface CourseLesson {
	id: string;
	module_id: string;
	title: string;
	slug: string;
	description: string | null;
	content: string;
	content_json: Record<string, unknown> | null;
	video_url: string | null;
	video_duration_seconds: number | null;
	sort_order: number;
	is_preview: boolean;
	created_at: string;
	updated_at: string;
}

export interface LessonProgress {
	id: string;
	user_id: string;
	lesson_id: string;
	completed: boolean;
	progress_seconds: number;
	completed_at: string | null;
	last_accessed_at: string;
}

export interface CourseWithModules extends Course {
	modules: ModuleWithLessons[];
	total_lessons: number;
	total_duration_seconds: number;
}

export interface ModuleWithLessons extends CourseModule {
	lessons: CourseLesson[];
}

export interface CourseListItem {
	id: string;
	title: string;
	slug: string;
	short_description: string | null;
	thumbnail_url: string | null;
	difficulty: string;
	instructor_name: string;
	price_cents: number;
	is_free: boolean;
	is_included_in_subscription: boolean;
	published: boolean;
	estimated_duration_minutes: number;
	total_lessons: number;
	created_at: string;
}

// ── Pricing Plans ─────────────────────────────────────────────────────

export interface PricingPlan {
	id: string;
	name: string;
	slug: string;
	description: string | null;
	stripe_price_id: string | null;
	stripe_product_id: string | null;
	amount_cents: number;
	currency: string;
	interval: 'month' | 'year' | 'one_time';
	interval_count: number;
	trial_days: number;
	/**
	 * When `true` (default), Stripe Checkout collects a card up front and
	 * charges after the trial. When `false`, the BFF passes
	 * `payment_method_collection: 'if_required'` so the user starts the
	 * trial without a card. Pair with a non-zero `trial_days` for a
	 * "free trial — no credit card required" SKU.
	 */
	collect_payment_method_at_checkout: boolean;
	features: string[];
	highlight_text: string | null;
	is_popular: boolean;
	is_active: boolean;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface CreatePricingPlanPayload {
	name: string;
	slug?: string;
	description?: string;
	stripe_price_id?: string;
	stripe_product_id?: string;
	amount_cents: number;
	currency?: string;
	interval?: string;
	interval_count?: number;
	trial_days?: number;
	features?: string[];
	highlight_text?: string;
	is_popular?: boolean;
	is_active?: boolean;
	sort_order?: number;
}

export type PricingStripeRolloutAudience =
	| 'linked_subscriptions_only'
	| 'linked_and_unlinked_legacy_same_cadence';

export interface PricingStripeRollout {
	push_to_stripe_subscriptions: boolean;
	audience: PricingStripeRolloutAudience;
	/**
	 * Skip subscriptions with `price_protection_enabled = TRUE` so grandfathered
	 * members keep their rate. Defaults to `true` server-side; explicitly
	 * setting `false` would bulldoze protected subscriptions and is not
	 * exposed in the admin UI.
	 */
	skip_price_protected?: boolean;
}

/** `GET /api/admin/pricing/plans/{id}/rollout-preview` — dry-run before commit */
export interface PricingRolloutPreview {
	total_in_audience: number;
	would_update: number;
	would_skip_grandfathered: number;
	current_amount_cents: number;
	currency: string;
}

export interface UpdatePricingPlanPayload {
	name?: string;
	slug?: string;
	description?: string;
	stripe_price_id?: string;
	stripe_product_id?: string;
	amount_cents?: number;
	currency?: string;
	interval?: string;
	interval_count?: number;
	trial_days?: number;
	features?: string[];
	highlight_text?: string;
	is_popular?: boolean;
	is_active?: boolean;
	sort_order?: number;
	stripe_rollout?: PricingStripeRollout;
}

export interface AdminStripeRolloutFailure {
	stripe_subscription_id: string;
	user_id: string;
	error: string;
}

export interface AdminStripeRolloutSummary {
	targeted: number;
	succeeded: number;
	skipped_grandfathered: number;
	failed: AdminStripeRolloutFailure[];
}

/** `PUT /api/admin/pricing/plans/{id}` — catalog row plus optional Stripe rollout stats */
export type AdminUpdatePricingPlanResponse = PricingPlan & {
	stripe_rollout?: AdminStripeRolloutSummary | null;
};

export interface PricingPlanPriceLogEntry {
	id: string;
	plan_name: string;
	old_amount_cents: number;
	new_amount_cents: number;
	changed_at: string;
	changed_by: string;
}

// ── Coupons ───────────────────────────────────────────────────────────

export type DiscountType = 'percentage' | 'fixed_amount' | 'free_trial';

export interface Coupon {
	id: string;
	code: string;
	description: string | null;
	discount_type: DiscountType;
	discount_value: number;
	min_purchase_cents: number | null;
	max_discount_cents: number | null;
	applies_to: 'all' | 'monthly' | 'annual' | 'course' | 'specific_plans';
	applicable_plan_ids: string[];
	applicable_course_ids: string[];
	usage_limit: number | null;
	usage_count: number;
	per_user_limit: number;
	starts_at: string | null;
	expires_at: string | null;
	is_active: boolean;
	stackable: boolean;
	first_purchase_only: boolean;
	stripe_coupon_id: string | null;
	stripe_promotion_code_id: string | null;
	created_by: string;
	created_at: string;
	updated_at: string;
}

export interface CouponValidationResponse {
	valid: boolean;
	coupon: Coupon | null;
	discount_amount_cents: number | null;
	message: string;
}

export interface BulkCouponPayload {
	count: number;
	prefix?: string | undefined;
	discount_type: DiscountType;
	discount_value: number;
	usage_limit?: number | undefined;
	expires_at?: string | undefined;
}

// ── Popups ────────────────────────────────────────────────────────────

export type PopupType = 'modal' | 'slide_in' | 'banner' | 'fullscreen' | 'floating_bar' | 'inline';
export type PopupTrigger =
	| 'on_load'
	| 'exit_intent'
	| 'scroll_percentage'
	| 'time_delay'
	| 'click'
	| 'manual'
	| 'inactivity';
export type PopupFrequency = 'every_time' | 'once_per_session' | 'once_ever' | 'custom';

export interface PopupElement {
	id: string;
	type:
		| 'heading'
		| 'text'
		| 'image'
		| 'input'
		| 'email'
		| 'textarea'
		| 'select'
		| 'checkbox'
		| 'radio'
		| 'button'
		| 'divider'
		| 'spacer';
	props: Record<string, unknown>;
	style?: Record<string, string>;
}

export interface PopupStyle {
	background: string;
	textColor: string;
	accentColor: string;
	borderRadius: string;
	maxWidth: string;
	animation: 'fade' | 'slide_up' | 'slide_down' | 'slide_left' | 'slide_right' | 'scale' | 'none';
	backdrop: boolean;
	backdropColor: string;
	padding?: string;
	shadow?: string;
}

export interface PopupTargetingRules {
	pages: string[];
	devices: ('desktop' | 'mobile' | 'tablet')[];
	userStatus: ('all' | 'logged_in' | 'logged_out' | 'member' | 'non_member')[];
}

export interface Popup {
	id: string;
	name: string;
	popup_type: PopupType;
	trigger_type: PopupTrigger;
	trigger_config: Record<string, unknown>;
	content_json: { elements: PopupElement[] };
	style_json: PopupStyle;
	targeting_rules: PopupTargetingRules;
	display_frequency: PopupFrequency;
	frequency_config: Record<string, unknown>;
	success_message: string | null;
	redirect_url: string | null;
	is_active: boolean;
	starts_at: string | null;
	expires_at: string | null;
	priority: number;
	created_by: string;
	created_at: string;
	updated_at: string;
}

export interface CreatePopupPayload {
	name: string;
	popup_type?: PopupType | undefined;
	trigger_type?: PopupTrigger | undefined;
	trigger_config?: Record<string, unknown> | undefined;
	content_json?: { elements: PopupElement[] } | undefined;
	style_json?: Partial<PopupStyle> | undefined;
	targeting_rules?: Partial<PopupTargetingRules> | undefined;
	display_frequency?: PopupFrequency | undefined;
	frequency_config?: Record<string, unknown> | undefined;
	success_message?: string | undefined;
	redirect_url?: string | undefined;
	is_active?: boolean | undefined;
	starts_at?: string | undefined;
	expires_at?: string | undefined;
	priority?: number | undefined;
}

export type UpdatePopupPayload = Partial<CreatePopupPayload>;

export interface PopupSubmission {
	id: string;
	popup_id: string;
	user_id: string | null;
	session_id: string | null;
	form_data: Record<string, unknown>;
	ip_address: string | null;
	user_agent: string | null;
	page_url: string | null;
	submitted_at: string;
}

export interface PopupAnalytics {
	popup_id: string;
	popup_name: string;
	total_impressions: number;
	total_closes: number;
	total_submissions: number;
	conversion_rate: number;
}

/**
 * Per-popup summary returned by `GET /api/admin/popups/analytics`.
 * Distinct from `PopupAnalytics` (single-popup detail view) — the
 * collection endpoint uses shorter field names and includes
 * `popup_type` / `is_active` for the admin index roster.
 */
export interface PopupAnalyticsSummary {
	popup_id: string;
	popup_name: string;
	popup_type: string;
	is_active: boolean;
	impressions: number;
	closes: number;
	submits: number;
	conversion_rate: number;
}
