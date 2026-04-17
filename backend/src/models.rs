use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// ── User ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub role: UserRole,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub position: Option<String>,
    pub website_url: Option<String>,
    pub twitter_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub youtube_url: Option<String>,
    pub instagram_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Member,
    Admin,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub position: Option<String>,
    pub website_url: Option<String>,
    pub twitter_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub youtube_url: Option<String>,
    pub instagram_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            name: u.name,
            role: u.role,
            avatar_url: u.avatar_url,
            bio: u.bio,
            position: u.position,
            website_url: u.website_url,
            twitter_url: u.twitter_url,
            linkedin_url: u.linkedin_url,
            youtube_url: u.youtube_url,
            instagram_url: u.instagram_url,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

// ── Password Reset ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}

// ── Subscription ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stripe_customer_id: String,
    pub stripe_subscription_id: String,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_plan", rename_all = "lowercase")]
pub enum SubscriptionPlan {
    Monthly,
    Annual,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_status", rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    PastDue,
    Trialing,
    Unpaid,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionStatusResponse {
    pub subscription: Option<Subscription>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct BillingPortalResponse {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct BillingPortalRequest {
    pub return_url: Option<String>,
}

// ── Watchlist ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Watchlist {
    pub id: Uuid,
    pub title: String,
    pub week_of: chrono::NaiveDate,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateWatchlistRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub week_of: chrono::NaiveDate,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWatchlistRequest {
    pub title: Option<String>,
    pub week_of: Option<chrono::NaiveDate>,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: Option<bool>,
}

// ── Watchlist Alert ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WatchlistAlert {
    pub id: Uuid,
    pub watchlist_id: Uuid,
    pub ticker: String,
    pub direction: TradeDirection,
    pub entry_zone: String,
    pub invalidation: String,
    pub profit_zones: Vec<String>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "trade_direction", rename_all = "lowercase")]
pub enum TradeDirection {
    Bullish,
    Bearish,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateAlertRequest {
    #[validate(length(min = 1, message = "Ticker is required"))]
    pub ticker: String,
    pub direction: TradeDirection,
    #[validate(length(min = 1, message = "Entry zone is required"))]
    pub entry_zone: String,
    #[validate(length(min = 1, message = "Invalidation is required"))]
    pub invalidation: String,
    pub profit_zones: Vec<String>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAlertRequest {
    pub ticker: Option<String>,
    pub direction: Option<TradeDirection>,
    pub entry_zone: Option<String>,
    pub invalidation: Option<String>,
    pub profit_zones: Option<Vec<String>>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
}

// ── Course Enrollment ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CourseEnrollment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: String,
    pub progress: i32,
    pub enrolled_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ── Refresh Token ───────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub used: bool,
}

// ── Watchlist with alerts (joined response) ─────────────────────────────

#[derive(Debug, Serialize)]
pub struct WatchlistWithAlerts {
    #[serde(flatten)]
    pub watchlist: Watchlist,
    pub alerts: Vec<WatchlistAlert>,
}

// ── Admin dashboard stats ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub total_members: i64,
    pub active_subscriptions: i64,
    pub monthly_subscriptions: i64,
    pub annual_subscriptions: i64,
    pub total_watchlists: i64,
    pub total_enrollments: i64,
    pub recent_members: Vec<UserResponse>,
}

// ── Blog Post ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "post_status", rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    PendingReview,
    Published,
    Private,
    Scheduled,
    Trash,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogPost {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_id: Option<Uuid>,
    pub status: PostStatus,
    pub pre_trash_status: Option<PostStatus>,
    pub trashed_at: Option<DateTime<Utc>>,
    pub visibility: String,
    pub password_hash: Option<String>,
    pub format: String,
    pub is_sticky: bool,
    pub allow_comments: bool,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub reading_time_minutes: i32,
    pub word_count: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BlogPostResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub author_position: Option<String>,
    pub author_bio: Option<String>,
    pub author_website: Option<String>,
    pub author_twitter: Option<String>,
    pub author_linkedin: Option<String>,
    pub author_youtube: Option<String>,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_url: Option<String>,
    pub status: PostStatus,
    pub pre_trash_status: Option<PostStatus>,
    pub trashed_at: Option<DateTime<Utc>>,
    pub visibility: String,
    pub is_password_protected: bool,
    pub format: String,
    pub is_sticky: bool,
    pub allow_comments: bool,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub reading_time_minutes: i32,
    pub word_count: i32,
    pub categories: Vec<BlogCategory>,
    pub tags: Vec<BlogTag>,
    pub meta: Vec<PostMeta>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BlogPostListItem {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub featured_image_url: Option<String>,
    pub status: PostStatus,
    pub format: String,
    pub is_sticky: bool,
    pub reading_time_minutes: i32,
    pub word_count: i32,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub categories: Vec<BlogCategory>,
    pub tags: Vec<BlogTag>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub slug: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_id: Option<Uuid>,
    pub status: Option<PostStatus>,
    pub visibility: Option<String>,
    pub is_sticky: Option<bool>,
    pub allow_comments: Option<bool>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub category_ids: Option<Vec<Uuid>>,
    pub tag_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub post_password: Option<String>,
    pub author_id: Option<Uuid>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_id: Option<Uuid>,
    pub status: Option<PostStatus>,
    pub visibility: Option<String>,
    pub is_sticky: Option<bool>,
    pub allow_comments: Option<bool>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub category_ids: Option<Vec<Uuid>>,
    pub tag_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub post_password: Option<String>,
    pub author_id: Option<Uuid>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostStatusRequest {
    pub status: PostStatus,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPostPasswordRequest {
    pub password: String,
}

// ── Post Meta ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct PostMeta {
    pub id: Uuid,
    pub post_id: Uuid,
    pub meta_key: String,
    pub meta_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostMetaRequest {
    pub meta_key: String,
    pub meta_value: String,
}

#[derive(Debug, Deserialize)]
pub struct PostListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<PostStatus>,
    pub author_id: Option<Uuid>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AutosaveRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
}

// ── Blog Category ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

// ── Blog Tag ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogTag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
}

// ── Blog Revision ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogRevision {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub revision_number: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RevisionResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub title: String,
    pub revision_number: i32,
    pub created_at: DateTime<Utc>,
}

// ── Media ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Media {
    pub id: Uuid,
    pub uploader_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub title: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub storage_path: String,
    pub url: String,
    pub focal_x: f64,
    pub focal_y: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMediaRequest {
    pub title: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub focal_x: Option<f64>,
    pub focal_y: Option<f64>,
}

// ── Pagination ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page();
        (page - 1) * per_page
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// ── Analytics ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyticsIngestEvent {
    pub event_type: String,
    pub path: String,
    pub referrer: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsIngestRequest {
    pub session_id: Uuid,
    pub events: Vec<AnalyticsIngestEvent>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsTimeBucket {
    pub date: String,
    pub page_views: i64,
    pub unique_sessions: i64,
    pub impressions: i64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsTopPage {
    pub path: String,
    pub views: i64,
    pub sessions: i64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsRecentSale {
    pub id: Uuid,
    pub event_type: String,
    pub amount_cents: i32,
    pub user_email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsCtrPoint {
    pub date: String,
    pub cta_id: String,
    pub impressions: i64,
    pub clicks: i64,
    pub ctr: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSummary {
    pub from: String,
    pub to: String,
    pub total_page_views: i64,
    pub total_sessions: i64,
    pub total_impressions: i64,
    /// Sessions with exactly one page view in the period (classic single-page session bounce).
    pub bounced_sessions: i64,
    /// Sessions with ≥1 page view in the period (denominator for bounce rate).
    pub bounce_eligible_sessions: i64,
    pub bounce_rate: f64,
    /// Estimated from active subscription counts × `pricing_plans` amounts (monthly + annual/12).
    pub mrr_cents: i64,
    pub arr_cents: i64,
    pub active_subscribers: i64,
    /// Sum of `sales_events.amount_cents` in `[from, to]`.
    pub period_revenue_cents: i64,
    pub time_series: Vec<AnalyticsTimeBucket>,
    pub top_pages: Vec<AnalyticsTopPage>,
    pub ctr_series: Vec<AnalyticsCtrPoint>,
    pub recent_sales: Vec<AnalyticsRecentSale>,
}

#[derive(Debug, Serialize)]
pub struct DailyRevenuePoint {
    pub date: String,
    pub revenue_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminRevenueResponse {
    pub data: Vec<DailyRevenuePoint>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsSummaryQuery {
    /// Inclusive start date `YYYY-MM-DD` (UTC).
    pub from: String,
    /// Exclusive end date `YYYY-MM-DD` (UTC), or inclusive depending on interpretation — we use end-exclusive window [from, to).
    pub to: String,
}

// ── Course ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Course {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: String,
    pub instructor_id: Uuid,
    pub price_cents: i32,
    pub currency: String,
    pub is_free: bool,
    pub is_included_in_subscription: bool,
    pub sort_order: i32,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub estimated_duration_minutes: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CourseModule {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CourseLesson {
    pub id: Uuid,
    pub module_id: Uuid,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub video_url: Option<String>,
    pub video_duration_seconds: Option<i32>,
    pub sort_order: i32,
    pub is_preview: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LessonProgress {
    pub id: Uuid,
    pub user_id: Uuid,
    pub lesson_id: Uuid,
    pub completed: bool,
    pub progress_seconds: i32,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCourseRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: Option<String>,
    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub is_free: Option<bool>,
    pub is_included_in_subscription: Option<bool>,
    pub sort_order: Option<i32>,
    pub published: Option<bool>,
    pub estimated_duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCourseRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: Option<String>,
    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub is_free: Option<bool>,
    pub is_included_in_subscription: Option<bool>,
    pub sort_order: Option<i32>,
    pub published: Option<bool>,
    pub estimated_duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateModuleRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModuleRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateLessonRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub video_url: Option<String>,
    pub video_duration_seconds: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_preview: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLessonRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub video_url: Option<String>,
    pub video_duration_seconds: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_preview: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLessonProgressRequest {
    pub progress_seconds: Option<i32>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CourseWithModules {
    #[serde(flatten)]
    pub course: Course,
    pub modules: Vec<ModuleWithLessons>,
    pub total_lessons: i64,
    pub total_duration_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct ModuleWithLessons {
    #[serde(flatten)]
    pub module: CourseModule,
    pub lessons: Vec<CourseLesson>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CourseListItem {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub difficulty: String,
    pub instructor_name: String,
    pub price_cents: i32,
    pub is_free: bool,
    pub is_included_in_subscription: bool,
    pub published: bool,
    pub estimated_duration_minutes: i32,
    pub total_lessons: i64,
    pub created_at: DateTime<Utc>,
}

// ── Pricing Plans ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PricingPlan {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: i32,
    pub currency: String,
    pub interval: String,
    pub interval_count: i32,
    pub trial_days: i32,
    pub features: serde_json::Value,
    pub highlight_text: Option<String>,
    pub is_popular: bool,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePricingPlanRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: i32,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub interval_count: Option<i32>,
    pub trial_days: Option<i32>,
    pub features: Option<serde_json::Value>,
    pub highlight_text: Option<String>,
    pub is_popular: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePricingPlanRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: Option<i32>,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub interval_count: Option<i32>,
    pub trial_days: Option<i32>,
    pub features: Option<serde_json::Value>,
    pub highlight_text: Option<String>,
    pub is_popular: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PricingChangeLog {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub field_changed: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: Uuid,
    pub changed_at: DateTime<Utc>,
}

// ── Coupons ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "discount_type", rename_all = "lowercase")]
pub enum DiscountType {
    Percentage,
    #[sqlx(rename = "fixed_amount")]
    FixedAmount,
    #[sqlx(rename = "free_trial")]
    FreeTrial,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Coupon {
    pub id: Uuid,
    pub code: String,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: rust_decimal::Decimal,
    pub min_purchase_cents: Option<i32>,
    pub max_discount_cents: Option<i32>,
    pub applies_to: String,
    pub applicable_plan_ids: Vec<Uuid>,
    pub applicable_course_ids: Vec<Uuid>,
    pub usage_limit: Option<i32>,
    pub usage_count: i32,
    pub per_user_limit: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub stackable: bool,
    pub first_purchase_only: bool,
    pub stripe_coupon_id: Option<String>,
    pub stripe_promotion_code_id: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCouponRequest {
    pub code: Option<String>,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: f64,
    pub min_purchase_cents: Option<i32>,
    pub max_discount_cents: Option<i32>,
    pub applies_to: Option<String>,
    pub applicable_plan_ids: Option<Vec<Uuid>>,
    pub applicable_course_ids: Option<Vec<Uuid>>,
    pub usage_limit: Option<i32>,
    pub per_user_limit: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub stackable: Option<bool>,
    pub first_purchase_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCouponRequest {
    pub description: Option<String>,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<f64>,
    pub min_purchase_cents: Option<i32>,
    pub max_discount_cents: Option<i32>,
    pub applies_to: Option<String>,
    pub applicable_plan_ids: Option<Vec<Uuid>>,
    pub applicable_course_ids: Option<Vec<Uuid>>,
    pub usage_limit: Option<i32>,
    pub per_user_limit: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub stackable: Option<bool>,
    pub first_purchase_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateCouponRequest {
    pub code: String,
    pub plan_id: Option<Uuid>,
    pub course_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CouponValidationResponse {
    pub valid: bool,
    pub coupon: Option<Coupon>,
    pub discount_amount_cents: Option<i32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CouponUsage {
    pub id: Uuid,
    pub coupon_id: Uuid,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub discount_applied_cents: i32,
    pub used_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BulkCouponRequest {
    pub count: i32,
    pub prefix: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: f64,
    pub usage_limit: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ── Popups ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Popup {
    pub id: Uuid,
    pub name: String,
    pub popup_type: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub content_json: serde_json::Value,
    pub style_json: serde_json::Value,
    pub targeting_rules: serde_json::Value,
    pub display_frequency: String,
    pub frequency_config: serde_json::Value,
    pub success_message: Option<String>,
    pub redirect_url: Option<String>,
    pub is_active: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub priority: i32,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePopupRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub popup_type: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub content_json: Option<serde_json::Value>,
    pub style_json: Option<serde_json::Value>,
    pub targeting_rules: Option<serde_json::Value>,
    pub display_frequency: Option<String>,
    pub frequency_config: Option<serde_json::Value>,
    pub success_message: Option<String>,
    pub redirect_url: Option<String>,
    pub is_active: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePopupRequest {
    pub name: Option<String>,
    pub popup_type: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub content_json: Option<serde_json::Value>,
    pub style_json: Option<serde_json::Value>,
    pub targeting_rules: Option<serde_json::Value>,
    pub display_frequency: Option<String>,
    pub frequency_config: Option<serde_json::Value>,
    pub success_message: Option<String>,
    pub redirect_url: Option<String>,
    pub is_active: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PopupSubmission {
    pub id: Uuid,
    pub popup_id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub form_data: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub page_url: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PopupSubmitRequest {
    pub popup_id: Uuid,
    pub session_id: Option<Uuid>,
    pub form_data: serde_json::Value,
    pub page_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PopupAnalytics {
    pub popup_id: Uuid,
    pub popup_name: String,
    pub total_impressions: i64,
    pub total_closes: i64,
    pub total_submissions: i64,
    pub conversion_rate: f64,
}

// Revenue analytics types (SalesEvent, MonthlyRevenueSnapshot, RevenueAnalytics,
// MonthlyRevenueSummary, PlanRevenueSummary, RevenueAnalyticsQuery) were deleted in
// FDN-01 because they had no callers. The underlying `sales_events` and
// `monthly_revenue_snapshots` tables (migration 014) remain. Event-sourced revenue
// analytics are scheduled for Phase 4 EC-12.
