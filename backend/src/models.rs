use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

// ── User ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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
    // ADM-02 lifecycle columns. All optional; semantically `None` means
    // "not in that state". `email_verified_at` is set by self-serve
    // verification or admin override.
    #[serde(default)]
    pub suspended_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub suspension_reason: Option<String>,
    #[serde(default)]
    pub banned_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub ban_reason: Option<String>,
    #[serde(default)]
    pub email_verified_at: Option<DateTime<Utc>>,
    // ADM-15 billing profile + temporary-suspension window (migration 079).
    #[serde(default)]
    pub billing_line1: Option<String>,
    #[serde(default)]
    pub billing_line2: Option<String>,
    #[serde(default)]
    pub billing_city: Option<String>,
    #[serde(default)]
    pub billing_state: Option<String>,
    #[serde(default)]
    pub billing_postal_code: Option<String>,
    #[serde(default)]
    pub billing_country: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub suspended_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, Hash, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Member,
    Author,
    Support,
    Admin,
}

impl UserRole {
    /// Parse the lowercase database string (mirrors `#[sqlx(rename_all = "lowercase")]`).
    ///
    /// Returns `None` for unknown labels so the caller can decide how to surface
    /// the error — typically via `AppError::Internal` when a JWT claim carries a
    /// role string the backend doesn't recognize.
    #[must_use]
    pub fn from_str_lower(s: &str) -> Option<Self> {
        match s {
            "member" => Some(Self::Member),
            "author" => Some(Self::Author),
            "support" => Some(Self::Support),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    /// Return the canonical lowercase database string — inverse of `from_str_lower`.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Author => "author",
            Self::Support => "support",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
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
    /// ADM-02: lifecycle state surfaced to the admin members UI. `None`
    /// for each timestamp means the account is not in that state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspension_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banned_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ban_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified_at: Option<DateTime<Utc>>,
    // ADM-15 billing profile (migration 079) — surfaced so the admin
    // detail page can render the same address Stripe holds for the
    // customer. `None` for either column means we never collected it
    // (e.g. accounts created before the column existed, or a checkout
    // that didn't request a billing address).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_line1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_line2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended_until: Option<DateTime<Utc>>,
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
            suspended_at: u.suspended_at,
            suspension_reason: u.suspension_reason,
            banned_at: u.banned_at,
            ban_reason: u.ban_reason,
            email_verified_at: u.email_verified_at,
            billing_line1: u.billing_line1,
            billing_line2: u.billing_line2,
            billing_city: u.billing_city,
            billing_state: u.billing_state,
            billing_postal_code: u.billing_postal_code,
            billing_country: u.billing_country,
            phone: u.phone,
            suspended_until: u.suspended_until,
        }
    }
}

// ── ADM-15: admin members lifecycle / profile DTOs ──────────────────────

/// Billing address payload — mirrors the Stripe `Address` shape so the
/// JSON we accept on `PATCH /api/admin/members/{id}` round-trips into
/// `Customer.address` with no field-mapping at the handler edge.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct BillingAddress {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// ISO 3166-1 alpha-2 country code. Validated case-insensitively;
    /// the handler normalises to upper-case before persisting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// `PATCH /api/admin/members/{id}` body. Every field is optional —
/// callers send only what they want to change. Missing keys leave the
/// existing column untouched; `null` is currently treated as
/// "no change" rather than "clear the field" so support workflows that
/// PATCH a single field don't accidentally wipe sibling columns.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateMemberRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub billing_address: Option<BillingAddress>,
}

/// `POST /api/admin/members/{id}/suspend` body. Combines the existing
/// `LifecycleRequest` (just a `reason`) with an optional `until`
/// timestamp that flips the suspension into a *timeout*.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SuspendMemberRequest {
    #[serde(default)]
    pub reason: Option<String>,
    /// When set, the suspension auto-lifts once `now() >= until`.
    /// Open-ended suspensions (no `until`) require a manual
    /// `unsuspend` call.
    #[serde(default)]
    pub until: Option<DateTime<Utc>>,
}

/// One row in the activity timeline rendered on the member detail page.
/// Mirrors `admin_actions` minus the `id` (the timeline doesn't link
/// out to the audit viewer yet) and with the metadata pre-serialised.
#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct MemberActivityEntry {
    pub action: String,
    pub actor_id: Uuid,
    pub actor_role: UserRole,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// One row in the recent-payment-failures list on the member detail page.
#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct MemberPaymentFailure {
    pub stripe_invoice_id: Option<String>,
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub attempt_count: i32,
    pub created_at: DateTime<Utc>,
}

/// Composite payload returned by `GET /api/admin/members/{id}/detail`.
/// Pre-bundles the member, their current subscription, and the recent
/// activity streams the detail UI renders so the SPA needs one round
/// trip per page load.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberDetailResponse {
    pub user: UserResponse,
    pub subscription: Option<Subscription>,
    pub activity: Vec<MemberActivityEntry>,
    pub payment_failures: Vec<MemberPaymentFailure>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    /// Optional during BFF rollout (Phase 1.3): when the SPA carries the
    /// refresh token via the `swings_refresh` httpOnly cookie, the JSON body
    /// is empty (`{}`) and this field is `None`. The handler reads from the
    /// cookie jar in that case. Legacy clients can still send the value here.
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

// ── Password Reset ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EmailVerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResendVerificationRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

// ── Subscription ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stripe_customer_id: String,
    pub stripe_subscription_id: String,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    /// When set, this subscription was purchased from a specific `pricing_plans`
    /// catalog row (via Checkout metadata → Stripe subscription metadata → webhooks).
    pub pricing_plan_id: Option<Uuid>,
    /// Price the member was promised at signup. Populated at checkout time from
    /// `pricing_plans.amount_cents`. NULL for pre-migration rows.
    pub grandfathered_price_cents: Option<i64>,
    /// ISO-4217 currency that goes with `grandfathered_price_cents`.
    pub grandfathered_currency: Option<String>,
    /// When TRUE the pricing rollout service skips this subscription regardless
    /// of audience setting — the member keeps their original price forever.
    pub price_protection_enabled: bool,
    /// When the subscription will be canceled. Set when the member (or admin)
    /// schedules a cancel-at-period-end; the row is otherwise NULL while the
    /// subscription is active. Mirror of `subscriptions.cancel_at` from
    /// migration `041_subscriptions_v2.sql`.
    pub cancel_at: Option<DateTime<Utc>>,
    /// When the subscription was paused via Stripe `pause_collection`. NULL
    /// while active. Mirror of `subscriptions.paused_at` from migration
    /// `041_subscriptions_v2.sql`.
    pub paused_at: Option<DateTime<Utc>>,
    /// When an open-pause should auto-lift. NULL means manual resume only —
    /// the member must call `/unpause` explicitly. Mirror of
    /// `subscriptions.pause_resumes_at` from `041_subscriptions_v2.sql`.
    pub pause_resumes_at: Option<DateTime<Utc>>,
    /// When the trial ends. NULL when the subscription is not in trial.
    /// Mirror of `subscriptions.trial_end` from `041_subscriptions_v2.sql`.
    pub trial_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "subscription_plan", rename_all = "lowercase")]
pub enum SubscriptionPlan {
    Monthly,
    Annual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "subscription_status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    /// Maps to `past_due` in Postgres (note the underscore — `rename_all
    /// = "snake_case"` produces it). Pre-2026-05-01 the derive used
    /// `rename_all = "lowercase"` which serialised this variant as
    /// `pastdue` and caused every read of a past_due row to 500 with
    /// `ColumnDecode { invalid value "past_due" for enum SubscriptionStatus }`.
    PastDue,
    /// `pause_collection` is in effect — the member is not paying right now
    /// and must NOT have premium access. Added to the Postgres enum in
    /// `057_subscription_status_paused.sql`; before 2026-05-01 the Rust
    /// enum did not enumerate it, which made the row undecodable from
    /// sqlx and caused 500s on `/api/member/subscription` whenever a
    /// paused subscription was the latest row for a user.
    Paused,
    Trialing,
    Unpaid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionStatusResponse {
    pub subscription: Option<Subscription>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BillingPortalResponse {
    pub url: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BillingPortalRequest {
    pub return_url: Option<String>,
}

// ── Watchlist ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateWatchlistRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub week_of: chrono::NaiveDate,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWatchlistRequest {
    pub title: Option<String>,
    pub week_of: Option<chrono::NaiveDate>,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: Option<bool>,
}

// ── Watchlist Alert ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, ToSchema)]
#[sqlx(type_name = "trade_direction", rename_all = "lowercase")]
pub enum TradeDirection {
    Bullish,
    Bearish,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct WatchlistWithAlerts {
    #[serde(flatten)]
    pub watchlist: Watchlist,
    pub alerts: Vec<WatchlistAlert>,
}

// ── Admin dashboard stats ───────────────────────────────────────────────

/// Selectable reporting window for `GET /api/admin/stats`.
///
/// All variants except `Custom` resolve to a window ending at `now` and
/// extending back the indicated duration (or to the start of the current UTC
/// year for `YearToDate`). `Custom` requires `from` + `to` `YYYY-MM-DD` query
/// params (inclusive `to`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardRange {
    // serde's snake_case transform doesn't add underscores between letters
    // and digits (`Last7Days` → `last7_days`), so we pin the wire labels to
    // the analytics-style names the frontend uses.
    #[serde(rename = "last_7_days")]
    Last7Days,
    #[default]
    #[serde(rename = "last_30_days")]
    Last30Days,
    #[serde(rename = "last_90_days")]
    Last90Days,
    YearToDate,
    Custom,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DashboardStatsQuery {
    /// Reporting window. Defaults to `last_30_days` when omitted.
    #[serde(default)]
    pub range: Option<DashboardRange>,
    /// Inclusive start `YYYY-MM-DD`. Required when `range=custom`, ignored otherwise.
    pub from: Option<String>,
    /// Inclusive end `YYYY-MM-DD`. Required when `range=custom`, ignored otherwise.
    pub to: Option<String>,
}

/// Per-window counts. The dashboard sends two of these — the selected window
/// and the immediately-preceding window of identical length — so the UI can
/// compute deltas without a second round-trip.
#[derive(Debug, Serialize, ToSchema)]
pub struct PeriodWindow {
    pub new_members: i64,
    pub new_subscriptions: i64,
    pub canceled_subscriptions: i64,
    pub new_enrollments: i64,
    pub new_watchlists: i64,
    pub revenue_cents: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminStats {
    // ── Lifetime totals (unchanged behavior — "as of now") ─────────────
    pub total_members: i64,
    pub active_subscriptions: i64,
    pub monthly_subscriptions: i64,
    pub annual_subscriptions: i64,
    pub total_watchlists: i64,
    pub total_enrollments: i64,
    pub recent_members: Vec<UserResponse>,

    // ── Period-scoped fields ───────────────────────────────────────────
    /// Echo of the resolved range so the client can rehydrate the picker.
    pub range: DashboardRange,
    /// Resolved start of the selected window (UTC, inclusive).
    pub from: DateTime<Utc>,
    /// Resolved end of the selected window (UTC, exclusive).
    pub to: DateTime<Utc>,
    /// Counts inside the selected window.
    pub period: PeriodWindow,
    /// Counts inside the immediately-preceding window of equal length, for delta math.
    pub previous_period: PeriodWindow,
}

// ── Blog Post ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePostStatusRequest {
    pub status: PostStatus,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyPostPasswordRequest {
    pub password: String,
}

// ── Post Meta ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone, ToSchema)]
pub struct PostMeta {
    pub id: Uuid,
    pub post_id: Uuid,
    pub meta_key: String,
    pub meta_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertPostMetaRequest {
    pub meta_key: String,
    pub meta_value: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PostListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<PostStatus>,
    pub author_id: Option<Uuid>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AutosaveRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
}

// ── Blog Category ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BlogCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

// ── Blog Tag ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BlogTag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTagRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
}

// ── Blog Revision ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMediaRequest {
    pub title: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub focal_x: Option<f64>,
    pub focal_y: Option<f64>,
}

// ── Pagination ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// ── Analytics ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnalyticsIngestEvent {
    pub event_type: String,
    pub path: String,
    pub referrer: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnalyticsIngestRequest {
    pub session_id: Uuid,
    pub events: Vec<AnalyticsIngestEvent>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsTimeBucket {
    pub date: String,
    pub page_views: i64,
    pub unique_sessions: i64,
    pub impressions: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsTopPage {
    pub path: String,
    pub views: i64,
    pub sessions: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsRecentSale {
    pub id: Uuid,
    pub event_type: String,
    pub amount_cents: i64,
    pub user_email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnalyticsCtrPoint {
    pub date: String,
    pub cta_id: String,
    pub impressions: i64,
    pub clicks: i64,
    pub ctr: f64,
}

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct DailyRevenuePoint {
    pub date: String,
    pub revenue_cents: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminRevenueResponse {
    pub data: Vec<DailyRevenuePoint>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AnalyticsSummaryQuery {
    /// Inclusive start date `YYYY-MM-DD` (UTC).
    pub from: String,
    /// Exclusive end date `YYYY-MM-DD` (UTC), or inclusive depending on interpretation — we use end-exclusive window [from, to).
    pub to: String,
}

// ── Course ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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
    pub price_cents: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CourseModule {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LessonProgress {
    pub id: Uuid,
    pub user_id: Uuid,
    pub lesson_id: Uuid,
    pub completed: bool,
    pub progress_seconds: i32,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCourseRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub is_free: Option<bool>,
    pub is_included_in_subscription: Option<bool>,
    pub sort_order: Option<i32>,
    pub published: Option<bool>,
    pub estimated_duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCourseRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub is_free: Option<bool>,
    pub is_included_in_subscription: Option<bool>,
    pub sort_order: Option<i32>,
    pub published: Option<bool>,
    pub estimated_duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateModuleRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateModuleRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateLessonProgressRequest {
    pub progress_seconds: Option<i32>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CourseWithModules {
    #[serde(flatten)]
    pub course: Course,
    pub modules: Vec<ModuleWithLessons>,
    pub total_lessons: i64,
    pub total_duration_seconds: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ModuleWithLessons {
    #[serde(flatten)]
    pub module: CourseModule,
    pub lessons: Vec<CourseLesson>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct CourseListItem {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub difficulty: String,
    pub instructor_name: String,
    pub price_cents: i64,
    pub is_free: bool,
    pub is_included_in_subscription: bool,
    pub published: bool,
    pub estimated_duration_minutes: i32,
    pub total_lessons: i64,
    pub created_at: DateTime<Utc>,
}

// ── Pricing Plans ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PricingPlan {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub interval: String,
    pub interval_count: i32,
    pub trial_days: i32,
    /// When `true` (default), Stripe Checkout collects a card up-front and
    /// charges after the trial. When `false`, the BFF passes
    /// `payment_method_collection: 'if_required'` so the member starts the
    /// trial without entering a card. Stripe will refuse to bill the
    /// auto-conversion at trial end unless they add one — net effect is
    /// "trial → silent auto-cancel if no card." Toggle per plan.
    pub collect_payment_method_at_checkout: bool,
    pub features: serde_json::Value,
    pub highlight_text: Option<String>,
    pub is_popular: bool,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePricingPlanRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: i64,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub interval_count: Option<i32>,
    pub trial_days: Option<i32>,
    pub collect_payment_method_at_checkout: Option<bool>,
    pub features: Option<serde_json::Value>,
    pub highlight_text: Option<String>,
    pub is_popular: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

/// Controls which existing subscriptions are targeted when pushing a catalog
/// price change to Stripe.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, Eq, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PricingStripeRolloutAudience {
    /// Only rows where `subscriptions.pricing_plan_id` matches the plan being
    /// edited. Safest default — cannot bleed into the wrong plan.
    #[default]
    LinkedSubscriptionsOnly,
    /// Same as linked, plus legacy rows with `pricing_plan_id IS NULL` whose
    /// `plan` enum (monthly vs annual) matches this catalog plan’s cadence.
    ///
    /// **Footgun:** if you operate more than one active monthly (or annual)
    /// catalog plan, unlinked subscribers cannot be disambiguated — prefer
    /// `linked_subscriptions_only` and ensure Checkout sends metadata.
    LinkedAndUnlinkedLegacySameCadence,
}

/// Optional Stripe rollout payload on `PUT /api/admin/pricing/plans/{id}`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(default)]
pub struct PricingStripeRollout {
    /// When `true`, after the catalog row is persisted the API updates each
    /// targeted Stripe subscription’s primary line item to the new amount or
    /// `stripe_price_id`. Requires an `Idempotency-Key` request header.
    pub push_to_stripe_subscriptions: bool,
    pub audience: PricingStripeRolloutAudience,
    /// When `true`, skip every subscription where `price_protection_enabled =
    /// TRUE` — those members keep their grandfathered rate. When `false` (the
    /// default) protected subscriptions are still skipped because the rollout
    /// service always respects `price_protection_enabled`.
    ///
    /// This field is informational for the request body — the service always
    /// honours the DB flag.  Setting it `false` does not override protection.
    pub skip_price_protected: bool,
}

impl Default for PricingStripeRollout {
    fn default() -> Self {
        Self {
            push_to_stripe_subscriptions: false,
            audience: PricingStripeRolloutAudience::LinkedSubscriptionsOnly,
            skip_price_protected: true,
        }
    }
}

/// Read-only preview of which subscriptions would be affected before the
/// operator commits a rollout. Returned by
/// `GET /api/admin/pricing/plans/{id}/rollout-preview`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PricingRolloutPreview {
    /// Total active/trialing subscriptions in the chosen audience.
    pub total_in_audience: usize,
    /// Subscriptions that WOULD be updated (not price-protected).
    pub would_update: usize,
    /// Subscriptions that would be SKIPPED because `price_protection_enabled`.
    pub would_skip_grandfathered: usize,
    /// Current plan amount for reference.
    pub current_amount_cents: i64,
    pub currency: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePricingPlanRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub interval_count: Option<i32>,
    pub trial_days: Option<i32>,
    pub collect_payment_method_at_checkout: Option<bool>,
    pub features: Option<serde_json::Value>,
    pub highlight_text: Option<String>,
    pub is_popular: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
    pub stripe_rollout: Option<PricingStripeRollout>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminStripeRolloutFailure {
    pub stripe_subscription_id: String,
    pub user_id: Uuid,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminStripeRolloutSummary {
    pub targeted: usize,
    pub succeeded: usize,
    /// Subscriptions skipped because `price_protection_enabled = TRUE`.
    pub skipped_grandfathered: usize,
    pub failed: Vec<AdminStripeRolloutFailure>,
}

/// Response for admin plan update — catalog row plus optional Stripe rollout stats.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUpdatePricingPlanResponse {
    #[serde(flatten)]
    pub plan: PricingPlan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe_rollout: Option<AdminStripeRolloutSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PricingPlanAmountChangeLogEntry {
    pub id: Uuid,
    pub plan_name: String,
    pub old_amount_cents: i64,
    pub new_amount_cents: i64,
    pub changed_at: DateTime<Utc>,
    pub changed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, ToSchema)]
#[sqlx(type_name = "discount_type", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum DiscountType {
    Percentage,
    #[sqlx(rename = "fixed_amount")]
    FixedAmount,
    #[sqlx(rename = "free_trial")]
    FreeTrial,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Coupon {
    pub id: Uuid,
    pub code: String,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    #[schema(value_type = String, format = "decimal")]
    pub discount_value: rust_decimal::Decimal,
    pub min_purchase_cents: Option<i64>,
    pub max_discount_cents: Option<i64>,
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCouponRequest {
    pub code: Option<String>,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: f64,
    pub min_purchase_cents: Option<i64>,
    pub max_discount_cents: Option<i64>,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCouponRequest {
    pub description: Option<String>,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<f64>,
    pub min_purchase_cents: Option<i64>,
    pub max_discount_cents: Option<i64>,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidateCouponRequest {
    pub code: String,
    pub plan_id: Option<Uuid>,
    pub course_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CouponValidationResponse {
    pub valid: bool,
    pub coupon: Option<Coupon>,
    pub discount_amount_cents: Option<i64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CouponUsage {
    pub id: Uuid,
    pub coupon_id: Uuid,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub discount_applied_cents: i64,
    pub used_at: DateTime<Utc>,
    /// ISO 4217 currency code (lowercase) the discount was denominated in
    /// at redemption time. Defaults to `'usd'` for rows seeded before
    /// migration 085. Frontend formats `discount_applied_cents` against
    /// this code with `Intl.NumberFormat`.
    pub currency: String,
    /// Optional FK to `orders.id` when the redemption was tied to a
    /// concrete order. NULL for subscription-only applications (the
    /// `apply-coupon` member endpoint never has an order id) and for
    /// legacy rows seeded before migration 085.
    pub order_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkCouponRequest {
    pub count: i32,
    pub prefix: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: f64,
    pub usage_limit: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Aggregate counters for the admin coupons dashboard
/// (`GET /api/admin/coupons/stats`).
///
/// Counts are non-overlapping in spirit but a coupon can satisfy multiple
/// buckets (e.g. an inactive expired one); each metric is computed from its
/// own predicate against the `coupons` / `coupon_usages` tables, so the sum
/// of `active + expired + scheduled` is not guaranteed to equal `total`.
///
/// The struct carries both the audit-plan canonical names
/// (`total_coupons`, `active_coupons`, `expired_coupons`,
/// `redemptions_total`, `redemptions_today`) and the legacy aliases
/// (`total`, `active`, `expired`, `redemption_count`, `active_count`,
/// `total_usages`) so the frontend page that already binds against the
/// legacy names keeps rendering after the Phase 4.5 rollout.
#[derive(Debug, Serialize, ToSchema)]
pub struct CouponStats {
    // ── Audit-plan spec field names ──
    /// Phase 4.5 spec — count of all rows in `coupons`.
    pub total_coupons: i64,
    /// Phase 4.5 spec — `is_active = TRUE AND (expires_at IS NULL OR expires_at > NOW())`.
    pub active_coupons: i64,
    /// Phase 4.5 spec — `expires_at` is non-null and in the past.
    pub expired_coupons: i64,
    /// Phase 4.5 spec — total redemptions across `coupon_usages` (lifetime).
    pub redemptions_total: i64,
    /// Phase 4.5 spec — redemptions whose `used_at >= today (UTC midnight)`.
    pub redemptions_today: i64,

    // ── Legacy aliases (kept so the existing admin page does not crash) ──
    /// Legacy alias for `total_coupons`.
    pub total: i64,
    /// Legacy alias for `active_coupons`.
    pub active: i64,
    /// Legacy alias for `active_coupons` — frontend reads this name.
    pub active_count: i64,
    /// Legacy alias for `expired_coupons`.
    pub expired: i64,
    /// `starts_at` is non-null and in the future. Carried for the legacy
    /// renderer; not required by the audit-plan spec.
    pub scheduled: i64,
    /// Legacy alias for `redemptions_total`.
    pub redemption_count: i64,
    /// Legacy alias for `redemptions_total` — frontend reads this name.
    pub total_usages: i64,
    /// Sum of `coupon_usages.discount_applied_cents` across every redemption.
    pub total_discount_cents: i64,
}

// ── Popups ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct PopupSubmitRequest {
    pub popup_id: Uuid,
    pub session_id: Option<Uuid>,
    pub form_data: serde_json::Value,
    pub page_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PopupAnalytics {
    pub popup_id: Uuid,
    pub popup_name: String,
    pub total_impressions: i64,
    pub total_closes: i64,
    pub total_submissions: i64,
    pub conversion_rate: f64,
}

/// Per-popup summary row returned by `GET /api/admin/popups/analytics`.
///
/// Distinct from [`PopupAnalytics`] (the single-popup detail view) because
/// the collection endpoint also exposes `popup_type` / `is_active` so the
/// admin index can render a roster without a second round-trip.
#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct PopupAnalyticsSummary {
    pub popup_id: Uuid,
    pub popup_name: String,
    pub popup_type: String,
    pub is_active: bool,
    pub impressions: i64,
    pub closes: i64,
    pub submits: i64,
    pub conversion_rate: f64,
}

// Revenue analytics types (SalesEvent, MonthlyRevenueSnapshot, RevenueAnalytics,
// MonthlyRevenueSummary, PlanRevenueSummary, RevenueAnalyticsQuery) were deleted in
// FDN-01 because they had no callers. The underlying `sales_events` and
// `monthly_revenue_snapshots` tables (migration 014) remain. Event-sourced revenue
// analytics are scheduled for Phase 4 EC-12.
