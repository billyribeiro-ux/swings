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
