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
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
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
    pub visibility: String,
    pub password_hash: Option<String>,
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
    pub title: String,
    pub slug: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_url: Option<String>,
    pub status: PostStatus,
    pub visibility: String,
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
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostStatusRequest {
    pub status: PostStatus,
}

#[derive(Debug, Deserialize)]
pub struct PostListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<PostStatus>,
    pub author_id: Option<Uuid>,
    pub category_slug: Option<String>,
    pub tag_slug: Option<String>,
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
    pub mime_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub storage_path: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMediaRequest {
    pub alt_text: Option<String>,
    pub caption: Option<String>,
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
        self.per_page.unwrap_or(20).min(100).max(1)
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
