use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;

/// Trim + lowercase so logins match Gmail-style case-insensitive addresses.
#[must_use]
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

// ── Users ───────────────────────────────────────────────────────────────

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    name: &str,
) -> Result<User, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, password_hash, name, role)
        VALUES ($1, $2, $3, $4, 'member')
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(password_hash)
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE lower(btrim(email)) = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_users(
    pool: &PgPool,
    offset: i64,
    limit: i64,
) -> Result<(Vec<User>, i64), sqlx::Error> {
    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    Ok((users, total.0))
}

pub async fn update_user_role(
    pool: &PgPool,
    user_id: Uuid,
    role: &UserRole,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
    )
    .bind(role)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// ADM-15: lift an expired temporary suspension on first read.
///
/// The `users.suspended_until` column carries the deadline for a
/// timeout-style suspension; if it is in the past on the next login
/// attempt, the account is auto-reactivated rather than waiting for an
/// operator to explicitly call the unsuspend endpoint. We update only
/// when both columns are populated to avoid clobbering an open-ended
/// suspension that has no `until`.
///
/// Returns the refreshed [`User`] when a row was reactivated, or `None`
/// when the input either had no expiry or its expiry is still in the
/// future. Caller can swap the original row out for the returned one.
pub async fn lift_expired_suspension(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
           SET suspended_at      = NULL,
               suspension_reason = NULL,
               suspended_until   = NULL,
               updated_at        = NOW()
         WHERE id = $1
           AND suspended_at    IS NOT NULL
           AND suspended_until IS NOT NULL
           AND suspended_until <= NOW()
         RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// ADM-15: PATCH-style profile update from the admin members surface.
///
/// Each parameter is `Option`, mirroring the wire shape of
/// [`crate::models::UpdateMemberRequest`]: `None` leaves the column
/// untouched, `Some(value)` overwrites it. The address fields are
/// passed individually rather than as a struct so the COALESCE pattern
/// keeps every other column intact.
#[allow(clippy::too_many_arguments)]
pub async fn update_member_profile(
    pool: &PgPool,
    user_id: Uuid,
    name: Option<&str>,
    email: Option<&str>,
    phone: Option<&str>,
    billing_line1: Option<&str>,
    billing_line2: Option<&str>,
    billing_city: Option<&str>,
    billing_state: Option<&str>,
    billing_postal_code: Option<&str>,
    billing_country: Option<&str>,
    address_was_provided: bool,
    email_verified_at: Option<Option<DateTime<Utc>>>,
) -> Result<User, sqlx::Error> {
    let normalised_email = email.map(normalize_email);
    // When the caller submitted an `email_verified_at` reset (Some(None)),
    // we explicitly clear the verification timestamp; when they submitted
    // a new value, we set it; when they passed `None`, we leave it
    // alone. Using `query_as!` would be tighter but we don't have
    // sqlx-offline metadata for these columns yet.
    let (verified_clear, verified_set): (bool, Option<DateTime<Utc>>) = match email_verified_at {
        Some(None) => (true, None),
        Some(Some(v)) => (false, Some(v)),
        None => (false, None),
    };

    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
           SET name                = COALESCE($1, name),
               email               = COALESCE($2, email),
               phone               = COALESCE($3, phone),
               billing_line1       = CASE WHEN $11 THEN $4  ELSE billing_line1       END,
               billing_line2       = CASE WHEN $11 THEN $5  ELSE billing_line2       END,
               billing_city        = CASE WHEN $11 THEN $6  ELSE billing_city        END,
               billing_state       = CASE WHEN $11 THEN $7  ELSE billing_state       END,
               billing_postal_code = CASE WHEN $11 THEN $8  ELSE billing_postal_code END,
               billing_country     = CASE WHEN $11 THEN $9  ELSE billing_country     END,
               email_verified_at   = CASE
                                       WHEN $12 THEN NULL
                                       WHEN $13::timestamptz IS NOT NULL THEN $13
                                       ELSE email_verified_at
                                     END,
               updated_at          = NOW()
         WHERE id = $10
         RETURNING *
        "#,
    )
    .bind(name)
    .bind(normalised_email.as_deref())
    .bind(phone)
    .bind(billing_line1)
    .bind(billing_line2)
    .bind(billing_city)
    .bind(billing_state)
    .bind(billing_postal_code)
    .bind(billing_country)
    .bind(user_id)
    .bind(address_was_provided)
    .bind(verified_clear)
    .bind(verified_set)
    .fetch_one(pool)
    .await
}

/// ADM-15: clear the lifecycle ban columns on a member row.
pub async fn clear_user_ban(pool: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
           SET banned_at  = NULL,
               ban_reason = NULL,
               updated_at = NOW()
         WHERE id = $1
         RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// ADM-15: clear the lifecycle suspension columns on a member row.
pub async fn clear_user_suspension(pool: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
           SET suspended_at      = NULL,
               suspension_reason = NULL,
               suspended_until   = NULL,
               updated_at        = NOW()
         WHERE id = $1
         RETURNING *
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// ADM-15: timeout-style suspension (open-ended when `until` is `None`).
///
/// Hardens [`handlers::admin_security::suspend_member`] with the
/// `suspended_until` deadline introduced in migration 079. The login
/// gate consults [`lift_expired_suspension`] on the next attempt so the
/// row reactivates lazily; we do not need a background sweeper.
pub async fn suspend_user_until(
    pool: &PgPool,
    user_id: Uuid,
    reason: Option<&str>,
    until: Option<DateTime<Utc>>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
           SET suspended_at      = NOW(),
               suspension_reason = $1,
               suspended_until   = $2,
               banned_at         = NULL,
               ban_reason        = NULL,
               updated_at        = NOW()
         WHERE id = $3
         RETURNING *
        "#,
    )
    .bind(reason)
    .bind(until)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// ADM-15: persist the billing address Stripe Checkout returned for the
/// customer. Called from the `checkout.session.completed` webhook so the
/// admin members surface always reflects the latest address Stripe
/// holds. All fields nullable — Stripe omits keys whose value is `null`.
#[allow(clippy::too_many_arguments)]
pub async fn update_user_checkout_profile(
    pool: &PgPool,
    user_id: Uuid,
    phone: Option<&str>,
    line1: Option<&str>,
    line2: Option<&str>,
    city: Option<&str>,
    state: Option<&str>,
    postal_code: Option<&str>,
    country: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
           SET phone               = COALESCE($1, phone),
               billing_line1       = COALESCE($2, billing_line1),
               billing_line2       = COALESCE($3, billing_line2),
               billing_city        = COALESCE($4, billing_city),
               billing_state       = COALESCE($5, billing_state),
               billing_postal_code = COALESCE($6, billing_postal_code),
               billing_country     = COALESCE($7, billing_country),
               updated_at          = NOW()
         WHERE id = $8
        "#,
    )
    .bind(phone)
    .bind(line1)
    .bind(line2)
    .bind(city)
    .bind(state)
    .bind(postal_code)
    .bind(country)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// ADM-15: recent admin actions targeting a single user, used by the
/// member detail page's activity timeline. Matches both `target_id`
/// strings stored as the user's UUID and any audit row that names the
/// user inside its `metadata.user_id` field (the subscription / order
/// audits do this), via a single SELECT … UNION query for index reuse.
pub async fn recent_admin_actions_for_user(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<crate::models::MemberActivityEntry>, sqlx::Error> {
    let id_text = user_id.to_string();
    sqlx::query_as::<_, crate::models::MemberActivityEntry>(
        r#"
        SELECT action,
               actor_id,
               actor_role,
               created_at,
               metadata
          FROM admin_actions
         WHERE (target_kind = 'user' AND target_id = $1)
            OR metadata ->> 'user_id' = $1
         ORDER BY created_at DESC
         LIMIT $2
        "#,
    )
    .bind(&id_text)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// ADM-15: dunning history for a single user, used by the member detail
/// page. Joins to `payment_failures` directly on `user_id`; rows where
/// the FK was set on insert (most modern rows) come back without the
/// indirect `subscription.user_id` join.
pub async fn recent_payment_failures_for_user(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<crate::models::MemberPaymentFailure>, sqlx::Error> {
    sqlx::query_as::<_, crate::models::MemberPaymentFailure>(
        r#"
        SELECT stripe_invoice_id,
               amount_cents,
               currency,
               failure_code,
               failure_message,
               attempt_count,
               created_at
          FROM payment_failures
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn seed_admin(
    pool: &PgPool,
    email: &str,
    password: &str,
    name: &str,
) -> anyhow::Result<()> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hash error: {e}"))?
        .to_string();

    let email = normalize_email(email);

    // New installs: insert with seeded password. Existing admin: keep their password — do not
    // overwrite on every server start (that used to reset people back to ADMIN_PASSWORD / defaults).
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, role)
        VALUES ($1, $2, $3, $4, 'admin')
        ON CONFLICT (email) DO UPDATE
            SET name = EXCLUDED.name,
                role = EXCLUDED.role,
                updated_at = NOW()
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(&password_hash)
    .bind(name)
    .execute(pool)
    .await?;

    tracing::info!(
        "Admin user seeded (password unchanged if email already existed): {}",
        email
    );
    Ok(())
}

/// Lifecycle filter for [`search_users`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatusFilter {
    /// Account that is neither suspended nor banned.
    Active,
    /// `users.suspended_at IS NOT NULL`.
    Suspended,
    /// `users.banned_at IS NOT NULL`.
    Banned,
    /// `users.email_verified_at IS NULL`.
    Unverified,
}

impl UserStatusFilter {
    /// Parse the wire-form (lowercase) used by the admin UI query string.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "banned" => Some(Self::Banned),
            "unverified" => Some(Self::Unverified),
            _ => None,
        }
    }
}

/// ADM-10: paginated, indexed search across the members table.
///
/// `query` is interpreted as a case-insensitive substring against
/// `email` and `name`; both columns have GIN trigram indexes so the
/// `ILIKE '%q%'` predicates resolve in milliseconds at scale. When
/// the input contains a `@`, the email predicate is anchored on
/// `lower(email)` for slightly better selectivity. Empty `query`
/// degrades gracefully to a paginated list (and reuses the
/// `(role, created_at DESC)` index when `role_filter` is set).
///
/// Returns `(rows, total_matching)` so the caller can render a
/// paged grid without a follow-up round trip.
pub async fn search_users(
    pool: &PgPool,
    query: Option<&str>,
    role_filter: Option<&UserRole>,
    status_filter: Option<UserStatusFilter>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<User>, i64), sqlx::Error> {
    // We intentionally build the WHERE clause inline rather than via
    // dynamic SQL: every fragment uses bind parameters, so there is
    // no string-interpolation surface for SQL injection. The number
    // of bind slots is fixed; unused slots are passed as `NULL` and
    // gated by their corresponding `IS NULL OR …` guard.
    let q = query.map(|s| s.trim()).filter(|s| !s.is_empty());
    let pattern = q.map(|s| format!("%{}%", s.to_lowercase()));

    let status_active = matches!(status_filter, Some(UserStatusFilter::Active));
    let status_suspended = matches!(status_filter, Some(UserStatusFilter::Suspended));
    let status_banned = matches!(status_filter, Some(UserStatusFilter::Banned));
    let status_unverified = matches!(status_filter, Some(UserStatusFilter::Unverified));

    // Predicates:
    //   $1: pattern (NULL → no text filter)
    //   $2: role  (NULL → no role filter)
    //   $3-$6: status flags (Booleans, exactly 0 or 1 may be true)
    //   $7: limit, $8: offset
    let where_sql = r#"
        WHERE ($1::text IS NULL
               OR lower(email) LIKE $1
               OR lower(name)  LIKE $1)
          AND ($2::user_role IS NULL OR role = $2)
          AND ( NOT $3 OR (suspended_at IS NULL AND banned_at IS NULL) )
          AND ( NOT $4 OR suspended_at IS NOT NULL )
          AND ( NOT $5 OR banned_at    IS NOT NULL )
          AND ( NOT $6 OR email_verified_at IS NULL )
    "#;

    let list_sql =
        format!("SELECT * FROM users {where_sql} ORDER BY created_at DESC LIMIT $7 OFFSET $8");
    let count_sql = format!("SELECT COUNT(*) FROM users {where_sql}");

    let users: Vec<User> = sqlx::query_as::<_, User>(&list_sql)
        .bind(pattern.as_deref())
        .bind(role_filter)
        .bind(status_active)
        .bind(status_suspended)
        .bind(status_banned)
        .bind(status_unverified)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    // Reuse the same parameter shape; drop the LIMIT/OFFSET binds.
    let total: (i64,) = sqlx::query_as(&count_sql)
        .bind(pattern.as_deref())
        .bind(role_filter)
        .bind(status_active)
        .bind(status_suspended)
        .bind(status_banned)
        .bind(status_unverified)
        .fetch_one(pool)
        .await?;

    Ok((users, total.0))
}

/// ADM-10: admin-flow user creation.
///
/// Distinct from [`create_user`] because:
///   * the admin can pre-pick the role (member / author / support);
///   * the password may be `None` (operator skipped a temporary
///     credential and will rely on the password-reset / invite link);
///   * the email-verified flag may be set (operators creating an
///     account on behalf of a real human typically don't want the
///     "verify your email" friction);
///   * inserts are constrained to non-admin roles to keep the seed
///     pathway explicit (privilege escalation requires a deliberate
///     `update_user_role`).
pub async fn admin_create_user(
    pool: &PgPool,
    email: &str,
    name: &str,
    role: &UserRole,
    password_hash: Option<&str>,
    email_verified: bool,
) -> Result<User, sqlx::Error> {
    let email = normalize_email(email);
    // We default to argon2-shaped placeholder when no password is
    // supplied; auth pathways check `users.password_hash` length and
    // an unverifiable hash will simply fail any login attempt — but
    // the column is NOT NULL so we cannot leave it blank.
    let placeholder = "!disabled:awaiting-invite!".to_string();
    let hash = password_hash.unwrap_or(&placeholder);

    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, password_hash, name, role, email_verified_at)
        VALUES ($1, $2, $3, $4, $5, CASE WHEN $6 THEN NOW() ELSE NULL END)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(hash)
    .bind(name)
    .bind(role)
    .bind(email_verified)
    .fetch_one(pool)
    .await
}

pub async fn recent_members(pool: &PgPool, limit: i64) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE role = 'member' ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

// ── Range-scoped admin dashboard counters ───────────────────────────────
//
// All five helpers below answer the same question: "how many rows landed in
// table X with timestamp column Y inside `[start, end_exclusive)`?". They are
// intentionally tiny (no pluggable predicates) so the call sites in
// `handlers::admin::dashboard_stats` read top-to-bottom like a checklist of
// the metrics the dashboard surfaces.
//
// Window semantics: half-open `[start, end_exclusive)` everywhere. That
// matches the existing analytics endpoint (`analytics_sales_revenue_total_cents`)
// and is what the frontend's range resolver computes.

pub async fn count_users_created_between(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE created_at >= $1 AND created_at < $2")
            .bind(start)
            .bind(end_exclusive)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

pub async fn count_subscriptions_created_between(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM subscriptions WHERE created_at >= $1 AND created_at < $2",
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Count subscriptions whose Stripe-recorded cancellation timestamp lands in
/// `[start, end_exclusive)`. We use `subscriptions.canceled_at` (set when
/// Stripe finalises the cancellation) rather than `cancel_at` (the *scheduled*
/// future cutoff) because the dashboard wants observed churn, not pending
/// churn.
pub async fn count_subscriptions_canceled_between(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM subscriptions WHERE canceled_at IS NOT NULL AND canceled_at >= $1 AND canceled_at < $2",
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Course enrollments use `enrolled_at` (per `001_initial.sql`) as the
/// creation timestamp; there is no separate `created_at` column on this
/// table.
pub async fn count_enrollments_created_between(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM course_enrollments WHERE enrolled_at >= $1 AND enrolled_at < $2",
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn count_watchlists_created_between(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM watchlists WHERE created_at >= $1 AND created_at < $2",
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

// ── Password Reset Tokens ───────────────────────────────────────────────

pub async fn create_password_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    // Invalidate any existing unused tokens for this user
    sqlx::query("UPDATE password_reset_tokens SET used = TRUE WHERE user_id = $1 AND used = FALSE")
        .bind(user_id)
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_password_reset_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<PasswordResetToken>, sqlx::Error> {
    sqlx::query_as::<_, PasswordResetToken>(
        "SELECT * FROM password_reset_tokens WHERE token_hash = $1 AND used = FALSE AND expires_at > NOW()"
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn mark_reset_token_used(pool: &PgPool, token_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE password_reset_tokens SET used = TRUE WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_user_password(
    pool: &PgPool,
    user_id: Uuid,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(password_hash)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Email Verification Tokens ───────────────────────────────────────────

pub async fn create_email_verification_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE email_verification_tokens SET used_at = NOW() WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_email_verification_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<EmailVerificationToken>, sqlx::Error> {
    sqlx::query_as::<_, EmailVerificationToken>(
        "SELECT * FROM email_verification_tokens WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn mark_email_verification_token_used(
    pool: &PgPool,
    token_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE email_verification_tokens SET used_at = NOW() WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_user_email_verified(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET email_verified_at = COALESCE(email_verified_at, NOW()), updated_at = NOW() WHERE id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Refresh Tokens ──────────────────────────────────────────────────────

pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
    family_id: Uuid,
    used: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, family_id, used) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(family_id)
    .bind(used)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_refresh_token_used(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE refresh_tokens SET used = true WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_refresh_tokens_by_family(
    pool: &PgPool,
    family_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE family_id = $1")
        .bind(family_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshToken>, sqlx::Error> {
    sqlx::query_as::<_, RefreshToken>(
        "SELECT * FROM refresh_tokens WHERE token_hash = $1 AND expires_at > NOW()",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn delete_user_refresh_tokens(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Failed login attempts (ADM-02) ──────────────────────────────────────

/// ADM-02: append a failed login row.
///
/// `reason` is one of the values listed in `056_user_lifecycle.sql`'s
/// CHECK constraint: `unknown_email`, `bad_password`, `suspended`,
/// `banned`, `rate_limited`. Best-effort — callers ignore the result and
/// log on error so a flaky audit insert never masks the user-facing 401.
pub async fn record_failed_login(
    pool: &PgPool,
    email: &str,
    ip: Option<std::net::IpAddr>,
    user_agent: Option<&str>,
    reason: &str,
) -> Result<(), sqlx::Error> {
    let email_normalized = normalize_email(email);
    let ip_text = ip.map(|i| i.to_string());
    sqlx::query(
        r#"INSERT INTO failed_login_attempts (email, ip_address, user_agent, reason)
           VALUES ($1, $2::inet, $3, $4)"#,
    )
    .bind(email_normalized)
    .bind(ip_text)
    .bind(user_agent)
    .bind(reason)
    .execute(pool)
    .await?;
    Ok(())
}

/// Attempt to record a webhook event as processed. Returns `true` when the
/// row was newly inserted (caller should process the event), `false` when it
/// was already claimed (caller short-circuits).
///
/// `source` namespaces the idempotency key per-provider — `"stripe"` for the
/// existing Stripe handler, `"resend"` for FDN-09's email delivery callbacks.
/// See `migrations/023_webhook_source.sql` for the composite-PK rationale.
pub async fn try_claim_webhook_event(
    pool: &PgPool,
    source: &str,
    event_id: &str,
    event_type: &str,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"INSERT INTO processed_webhook_events (source, event_id, event_type)
           VALUES ($1, $2, $3)
           ON CONFLICT (source, event_id) DO NOTHING"#,
    )
    .bind(source)
    .bind(event_id)
    .bind(event_type)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Back-compat alias: the Stripe handler predates the multi-source column;
/// keeping the old name as a thin shim avoids churn in that handler and
/// lets us thread `source='stripe'` without touching the call site.
pub async fn try_claim_stripe_webhook_event(
    pool: &PgPool,
    event_id: &str,
    event_type: &str,
) -> Result<bool, sqlx::Error> {
    try_claim_webhook_event(pool, "stripe", event_id, event_type).await
}

pub async fn cleanup_old_stripe_webhook_events(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM processed_webhook_events WHERE processed_at < NOW() - INTERVAL '30 days'",
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

// ── Subscriptions ───────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn upsert_subscription(
    pool: &PgPool,
    user_id: Uuid,
    stripe_customer_id: &str,
    stripe_subscription_id: &str,
    plan: &SubscriptionPlan,
    status: &SubscriptionStatus,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    pricing_plan_id: Option<Uuid>,
) -> Result<Subscription, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        r#"
        INSERT INTO subscriptions (
            id, user_id, stripe_customer_id, stripe_subscription_id,
            plan, status, current_period_start, current_period_end, pricing_plan_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (stripe_subscription_id)
        DO UPDATE SET
            status = EXCLUDED.status,
            plan = EXCLUDED.plan,
            current_period_start = EXCLUDED.current_period_start,
            current_period_end = EXCLUDED.current_period_end,
            pricing_plan_id = COALESCE(EXCLUDED.pricing_plan_id, subscriptions.pricing_plan_id),
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(stripe_customer_id)
    .bind(stripe_subscription_id)
    .bind(plan)
    .bind(status)
    .bind(period_start)
    .bind(period_end)
    .bind(pricing_plan_id)
    .fetch_one(pool)
    .await
}

/// Active or trialing subscriptions considered for a catalog price rollout.
pub async fn list_subscriptions_for_pricing_rollout(
    pool: &PgPool,
    plan_id: Uuid,
    include_legacy_same_cadence: bool,
    cadence: &SubscriptionPlan,
) -> Result<Vec<Subscription>, sqlx::Error> {
    if include_legacy_same_cadence {
        sqlx::query_as::<_, Subscription>(
            r#"
            SELECT *
            FROM subscriptions
            WHERE status IN ('active', 'trialing')
              AND (
                    pricing_plan_id = $1
                 OR (pricing_plan_id IS NULL AND plan = $2)
                  )
            ORDER BY created_at ASC
            "#,
        )
        .bind(plan_id)
        .bind(cadence)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, Subscription>(
            r#"
            SELECT *
            FROM subscriptions
            WHERE status IN ('active', 'trialing')
              AND pricing_plan_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(plan_id)
        .fetch_all(pool)
        .await
    }
}

pub async fn find_subscription_by_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_subscription_by_stripe_id(
    pool: &PgPool,
    stripe_sub_id: &str,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE stripe_subscription_id = $1",
    )
    .bind(stripe_sub_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_user_by_stripe_customer(
    pool: &PgPool,
    customer_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u JOIN subscriptions s ON u.id = s.user_id WHERE s.stripe_customer_id = $1 LIMIT 1",
    )
    .bind(customer_id)
    .fetch_optional(pool)
    .await
}

pub async fn count_active_subscriptions(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE status = 'active'")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn count_subscriptions_by_plan(
    pool: &PgPool,
    plan: &SubscriptionPlan,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE plan = $1 AND status = 'active'")
            .bind(plan)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Count subscriptions in a given status (literal — accepts the enum
/// label as stored in Postgres so callers can ask about `paused` without
/// extending the Rust [`SubscriptionStatus`] enum).
pub async fn count_subscriptions_by_status(
    pool: &PgPool,
    status: &str,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE status::text = $1")
        .bind(status)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// One row in the admin subscriptions list. Joins `subscriptions` to
/// `users` (for the member display fields) and `pricing_plans` (for
/// the per-row amount when the subscription was bought from a specific
/// catalog row); when no catalog row is linked we fall back to the
/// public default for the plan cadence in the handler.
#[derive(Debug, sqlx::FromRow)]
pub struct AdminSubscriptionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: String,
    pub user_name: String,
    pub stripe_subscription_id: String,
    pub stripe_customer_id: String,
    pub plan: SubscriptionPlan,
    /// Read as text so the row tolerates the `paused` label that the
    /// Postgres `subscription_status` enum gained in
    /// `057_subscription_status_paused.sql` but the Rust
    /// [`SubscriptionStatus`] enum does not yet enumerate.
    pub status: String,
    pub plan_amount_cents: Option<i32>,
    pub plan_name: Option<String>,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub cancel_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub canceled_at: Option<DateTime<Utc>>,
}

/// List subscriptions for the admin dashboard with pagination and
/// optional `search` (matched against `users.email`) / `status` /
/// `plan` filters. Returns the page rows plus a total count for the
/// same filter set so the caller can render pagination.
///
/// `search` is matched as a case-insensitive substring (`ILIKE %q%`).
/// `status` and `plan` accept the lowercase enum labels as stored in
/// Postgres (`active|past_due|canceled|trialing|unpaid|paused` and
/// `monthly|annual`); invalid labels degrade to the unfiltered set
/// rather than raising — the handler should validate before calling.
pub async fn list_subscriptions_admin(
    pool: &PgPool,
    page: i64,
    per_page: i64,
    search: Option<&str>,
    status: Option<&str>,
    plan: Option<&str>,
) -> Result<(Vec<AdminSubscriptionRow>, i64), sqlx::Error> {
    let offset = (page - 1).max(0) * per_page;
    let search_pat = search.map(|s| format!("%{}%", s.replace(['%', '_'], "")));

    let rows = sqlx::query_as::<_, AdminSubscriptionRow>(
        r#"
        SELECT
            s.id,
            s.user_id,
            u.email                  AS user_email,
            u.name                   AS user_name,
            s.stripe_subscription_id,
            s.stripe_customer_id,
            s.plan,
            s.status::text           AS status,
            pp.amount_cents          AS plan_amount_cents,
            pp.name                  AS plan_name,
            s.current_period_start,
            s.current_period_end,
            s.cancel_at,
            s.created_at,
            s.canceled_at
        FROM subscriptions s
        JOIN users u           ON u.id = s.user_id
        LEFT JOIN pricing_plans pp ON pp.id = s.pricing_plan_id
        WHERE ($1::text IS NULL OR u.email ILIKE $1)
          AND ($2::text IS NULL OR s.status::text = $2)
          AND ($3::text IS NULL OR s.plan::text   = $3)
        ORDER BY s.created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(search_pat.as_deref())
    .bind(status)
    .bind(plan)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM subscriptions s
        JOIN users u ON u.id = s.user_id
        WHERE ($1::text IS NULL OR u.email ILIKE $1)
          AND ($2::text IS NULL OR s.status::text = $2)
          AND ($3::text IS NULL OR s.plan::text   = $3)
        "#,
    )
    .bind(search_pat.as_deref())
    .bind(status)
    .bind(plan)
    .fetch_one(pool)
    .await?;

    Ok((rows, total.0))
}

// ── Watchlists ──────────────────────────────────────────────────────────

pub async fn create_watchlist(
    pool: &PgPool,
    title: &str,
    week_of: NaiveDate,
    video_url: Option<&str>,
    notes: Option<&str>,
    published: bool,
) -> Result<Watchlist, sqlx::Error> {
    let published_at = if published { Some(Utc::now()) } else { None };

    sqlx::query_as::<_, Watchlist>(
        r#"
        INSERT INTO watchlists (id, title, week_of, video_url, notes, published, published_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(title)
    .bind(week_of)
    .bind(video_url)
    .bind(notes)
    .bind(published)
    .bind(published_at)
    .fetch_one(pool)
    .await
}

pub async fn update_watchlist(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateWatchlistRequest,
) -> Result<Watchlist, sqlx::Error> {
    let existing = sqlx::query_as::<_, Watchlist>("SELECT * FROM watchlists WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let week_of = req.week_of.unwrap_or(existing.week_of);
    let video_url = req.video_url.as_deref().or(existing.video_url.as_deref());
    let notes = req.notes.as_deref().or(existing.notes.as_deref());
    let published = req.published.unwrap_or(existing.published);
    let published_at = if published && existing.published_at.is_none() {
        Some(Utc::now())
    } else {
        existing.published_at
    };

    sqlx::query_as::<_, Watchlist>(
        r#"
        UPDATE watchlists SET title = $1, week_of = $2, video_url = $3, notes = $4, published = $5, published_at = $6, updated_at = NOW()
        WHERE id = $7 RETURNING *
        "#,
    )
    .bind(title)
    .bind(week_of)
    .bind(video_url)
    .bind(notes)
    .bind(published)
    .bind(published_at)
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_watchlist(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM watchlists WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_watchlist(pool: &PgPool, id: Uuid) -> Result<Option<Watchlist>, sqlx::Error> {
    sqlx::query_as::<_, Watchlist>("SELECT * FROM watchlists WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_watchlists(
    pool: &PgPool,
    offset: i64,
    limit: i64,
    published_only: bool,
) -> Result<(Vec<Watchlist>, i64), sqlx::Error> {
    let (watchlists, total) = if published_only {
        let wl = sqlx::query_as::<_, Watchlist>(
            "SELECT * FROM watchlists WHERE published = true ORDER BY week_of DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let t: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM watchlists WHERE published = true")
            .fetch_one(pool)
            .await?;
        (wl, t.0)
    } else {
        let wl = sqlx::query_as::<_, Watchlist>(
            "SELECT * FROM watchlists ORDER BY week_of DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let t: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM watchlists")
            .fetch_one(pool)
            .await?;
        (wl, t.0)
    };

    Ok((watchlists, total))
}

pub async fn count_watchlists(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM watchlists")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// ── Watchlist Alerts ────────────────────────────────────────────────────

pub async fn create_alert(
    pool: &PgPool,
    watchlist_id: Uuid,
    req: &CreateAlertRequest,
) -> Result<WatchlistAlert, sqlx::Error> {
    sqlx::query_as::<_, WatchlistAlert>(
        r#"
        INSERT INTO watchlist_alerts (id, watchlist_id, ticker, direction, entry_zone, invalidation, profit_zones, notes, chart_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(watchlist_id)
    .bind(&req.ticker)
    .bind(&req.direction)
    .bind(&req.entry_zone)
    .bind(&req.invalidation)
    .bind(&req.profit_zones)
    .bind(req.notes.as_deref())
    .bind(req.chart_url.as_deref())
    .fetch_one(pool)
    .await
}

pub async fn get_alerts_for_watchlist(
    pool: &PgPool,
    watchlist_id: Uuid,
) -> Result<Vec<WatchlistAlert>, sqlx::Error> {
    sqlx::query_as::<_, WatchlistAlert>(
        "SELECT * FROM watchlist_alerts WHERE watchlist_id = $1 ORDER BY created_at ASC",
    )
    .bind(watchlist_id)
    .fetch_all(pool)
    .await
}

pub async fn update_alert(
    pool: &PgPool,
    alert_id: Uuid,
    req: &UpdateAlertRequest,
) -> Result<WatchlistAlert, sqlx::Error> {
    let existing =
        sqlx::query_as::<_, WatchlistAlert>("SELECT * FROM watchlist_alerts WHERE id = $1")
            .bind(alert_id)
            .fetch_one(pool)
            .await?;

    sqlx::query_as::<_, WatchlistAlert>(
        r#"
        UPDATE watchlist_alerts SET
            ticker = $1, direction = $2, entry_zone = $3, invalidation = $4,
            profit_zones = $5, notes = $6, chart_url = $7
        WHERE id = $8 RETURNING *
        "#,
    )
    .bind(req.ticker.as_deref().unwrap_or(&existing.ticker))
    .bind(req.direction.as_ref().unwrap_or(&existing.direction))
    .bind(req.entry_zone.as_deref().unwrap_or(&existing.entry_zone))
    .bind(
        req.invalidation
            .as_deref()
            .unwrap_or(&existing.invalidation),
    )
    .bind(req.profit_zones.as_ref().unwrap_or(&existing.profit_zones))
    .bind(req.notes.as_deref().or(existing.notes.as_deref()))
    .bind(req.chart_url.as_deref().or(existing.chart_url.as_deref()))
    .bind(alert_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_alert(pool: &PgPool, alert_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM watchlist_alerts WHERE id = $1")
        .bind(alert_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Course Enrollments ──────────────────────────────────────────────────

// `enroll_user` was deleted in FDN-01 (no callers). When Phase 4 EC-10 adds the
// membership-driven enrollment path, it should provide its own enroll function
// alongside the membership grant logic rather than resurrecting this one.

pub async fn get_user_enrollments(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CourseEnrollment>, sqlx::Error> {
    sqlx::query_as::<_, CourseEnrollment>(
        "SELECT * FROM course_enrollments WHERE user_id = $1 ORDER BY enrolled_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn update_course_progress(
    pool: &PgPool,
    user_id: Uuid,
    course_id: &str,
    progress: i32,
) -> Result<CourseEnrollment, sqlx::Error> {
    let completed_at = if progress >= 100 {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query_as::<_, CourseEnrollment>(
        r#"
        UPDATE course_enrollments SET progress = $1, completed_at = $2
        WHERE user_id = $3 AND course_id = $4
        RETURNING *
        "#,
    )
    .bind(progress)
    .bind(completed_at)
    .bind(user_id)
    .bind(course_id)
    .fetch_one(pool)
    .await
}

pub async fn count_enrollments(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM course_enrollments")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// ── Blog Posts ─────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn compute_word_count(html: &str) -> i32 {
    let text: String = html
        .chars()
        .fold((String::new(), false), |(mut out, in_tag), c| {
            if c == '<' {
                (out, true)
            } else if c == '>' {
                out.push(' ');
                (out, false)
            } else if !in_tag {
                out.push(c);
                (out, false)
            } else {
                (out, true)
            }
        })
        .0;
    text.split_whitespace().count() as i32
}

fn compute_reading_time(word_count: i32) -> i32 {
    (word_count as f64 / 238.0).ceil() as i32
}

pub async fn create_blog_post(
    pool: &PgPool,
    author_id: Uuid,
    req: &CreatePostRequest,
    password_hash: Option<&str>,
) -> Result<BlogPost, sqlx::Error> {
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.title));
    let content = req.content.as_deref().unwrap_or("");
    let wc = compute_word_count(content);
    let rt = compute_reading_time(wc);
    let status = req.status.clone().unwrap_or(PostStatus::Draft);
    let published_at = if status == PostStatus::Published {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query_as::<_, BlogPost>(
        r#"
        INSERT INTO blog_posts (
            id, author_id, title, slug, content, content_json, excerpt,
            featured_image_id, status, visibility, password_hash, format, is_sticky, allow_comments,
            meta_title, meta_description, canonical_url, og_image_url,
            reading_time_minutes, word_count, scheduled_at, published_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, $9, $10, $11, $12, $13, $14,
            $15, $16, $17, $18,
            $19, $20, $21, $22
        ) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(author_id)
    .bind(&req.title)
    .bind(&slug)
    .bind(content)
    .bind(&req.content_json)
    .bind(req.excerpt.as_deref().unwrap_or(""))
    .bind(req.featured_image_id)
    .bind(&status)
    .bind(req.visibility.as_deref().unwrap_or("public"))
    .bind(password_hash)
    .bind(req.format.as_deref().unwrap_or("standard"))
    .bind(req.is_sticky.unwrap_or(false))
    .bind(req.allow_comments.unwrap_or(true))
    .bind(req.meta_title.as_deref().unwrap_or(""))
    .bind(req.meta_description.as_deref().unwrap_or(""))
    .bind(req.canonical_url.as_deref().unwrap_or(""))
    .bind(req.og_image_url.as_deref().unwrap_or(""))
    .bind(rt)
    .bind(wc)
    .bind(req.scheduled_at)
    .bind(published_at)
    .fetch_one(pool)
    .await
}

pub async fn update_blog_post(
    pool: &PgPool,
    post_id: Uuid,
    req: &UpdatePostRequest,
    password_hash_update: Option<Option<String>>,
    author_id_update: Option<Uuid>,
) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| existing.slug.clone());
    let content = req.content.as_deref().unwrap_or(&existing.content);
    let wc = compute_word_count(content);
    let rt = compute_reading_time(wc);
    let status = req.status.clone().unwrap_or(existing.status.clone());
    let published_at = if status == PostStatus::Published && existing.published_at.is_none() {
        Some(Utc::now())
    } else {
        existing.published_at
    };
    let (pre_trash_status, trashed_at): (Option<PostStatus>, Option<DateTime<Utc>>) = match &status
    {
        PostStatus::Trash if existing.status != PostStatus::Trash => {
            (Some(existing.status.clone()), Some(Utc::now()))
        }
        PostStatus::Trash => (existing.pre_trash_status, existing.trashed_at),
        _ => (None, None),
    };
    let new_password_hash = match password_hash_update {
        Some(v) => v,
        None => existing.password_hash.clone(),
    };
    let new_author_id = author_id_update.unwrap_or(existing.author_id);

    sqlx::query_as::<_, BlogPost>(
        r#"
        UPDATE blog_posts SET
            title = $1, slug = $2, content = $3, content_json = $4, excerpt = $5,
            featured_image_id = $6, status = $7, visibility = $8, password_hash = $9,
            format = $10, is_sticky = $11, allow_comments = $12, meta_title = $13,
            meta_description = $14, canonical_url = $15, og_image_url = $16,
            reading_time_minutes = $17, word_count = $18, scheduled_at = $19, published_at = $20,
            pre_trash_status = $21, trashed_at = $22,
            author_id = $23, updated_at = NOW()
        WHERE id = $24 RETURNING *
        "#,
    )
    .bind(title)
    .bind(&slug)
    .bind(content)
    .bind(req.content_json.as_ref().or(existing.content_json.as_ref()))
    .bind(
        req.excerpt
            .as_deref()
            .unwrap_or(existing.excerpt.as_deref().unwrap_or("")),
    )
    .bind(req.featured_image_id.or(existing.featured_image_id))
    .bind(&status)
    .bind(req.visibility.as_deref().unwrap_or(&existing.visibility))
    .bind(new_password_hash.as_deref())
    .bind(req.format.as_deref().unwrap_or(&existing.format))
    .bind(req.is_sticky.unwrap_or(existing.is_sticky))
    .bind(req.allow_comments.unwrap_or(existing.allow_comments))
    .bind(
        req.meta_title
            .as_deref()
            .unwrap_or(existing.meta_title.as_deref().unwrap_or("")),
    )
    .bind(
        req.meta_description
            .as_deref()
            .unwrap_or(existing.meta_description.as_deref().unwrap_or("")),
    )
    .bind(
        req.canonical_url
            .as_deref()
            .unwrap_or(existing.canonical_url.as_deref().unwrap_or("")),
    )
    .bind(
        req.og_image_url
            .as_deref()
            .unwrap_or(existing.og_image_url.as_deref().unwrap_or("")),
    )
    .bind(rt)
    .bind(wc)
    .bind(req.scheduled_at.or(existing.scheduled_at))
    .bind(published_at)
    .bind(pre_trash_status)
    .bind(trashed_at)
    .bind(new_author_id)
    .bind(post_id)
    .fetch_one(pool)
    .await
}

pub async fn autosave_blog_post(
    pool: &PgPool,
    post_id: Uuid,
    req: &AutosaveRequest,
) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let content = req.content.as_deref().unwrap_or(&existing.content);
    let wc = compute_word_count(content);
    let rt = compute_reading_time(wc);

    sqlx::query_as::<_, BlogPost>(
        r#"
        UPDATE blog_posts SET title = $1, content = $2, content_json = $3,
            reading_time_minutes = $4, word_count = $5, updated_at = NOW()
        WHERE id = $6 RETURNING *
        "#,
    )
    .bind(title)
    .bind(content)
    .bind(req.content_json.as_ref().or(existing.content_json.as_ref()))
    .bind(rt)
    .bind(wc)
    .bind(post_id)
    .fetch_one(pool)
    .await
}

/// Changes status for posts that are not in the trash. Does not handle `trash` or restore — use
/// [`move_post_to_trash`] / [`restore_post_from_trash`] from handlers instead.
pub async fn update_post_status(
    pool: &PgPool,
    post_id: Uuid,
    status: &PostStatus,
) -> Result<BlogPost, sqlx::Error> {
    debug_assert!(*status != PostStatus::Trash);
    let published_at_expr = if *status == PostStatus::Published {
        "COALESCE(published_at, NOW())"
    } else {
        "published_at"
    };

    let q = format!(
        "UPDATE blog_posts SET status = $1, published_at = {}, updated_at = NOW() WHERE id = $2 RETURNING *",
        published_at_expr
    );

    sqlx::query_as::<_, BlogPost>(&q)
        .bind(status)
        .bind(post_id)
        .fetch_one(pool)
        .await
}

pub async fn move_post_to_trash(pool: &PgPool, post_id: Uuid) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;
    if existing.status == PostStatus::Trash {
        return Ok(existing);
    }
    let prev = existing.status.clone();
    sqlx::query_as::<_, BlogPost>(
        r#"
        UPDATE blog_posts SET
            status = 'trash',
            pre_trash_status = $1,
            trashed_at = NOW(),
            updated_at = NOW()
        WHERE id = $2
        RETURNING *
        "#,
    )
    .bind(prev)
    .bind(post_id)
    .fetch_one(pool)
    .await
}

pub async fn restore_post_from_trash(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;
    if existing.status != PostStatus::Trash {
        return Ok(existing);
    }
    let new_status = existing
        .pre_trash_status
        .clone()
        .unwrap_or(PostStatus::Draft);
    let published_at_expr = if new_status == PostStatus::Published {
        "COALESCE(published_at, NOW())"
    } else {
        "published_at"
    };
    let q = format!(
        "UPDATE blog_posts SET status = $1, pre_trash_status = NULL, trashed_at = NULL, published_at = {}, updated_at = NOW() WHERE id = $2 RETURNING *",
        published_at_expr
    );
    sqlx::query_as::<_, BlogPost>(&q)
        .bind(new_status)
        .bind(post_id)
        .fetch_one(pool)
        .await
}

pub async fn delete_blog_post(pool: &PgPool, post_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_blog_post(pool: &PgPool, post_id: Uuid) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_blog_post_by_slug(
    pool: &PgPool,
    slug: &str,
) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        "SELECT * FROM blog_posts WHERE slug = $1 AND status = 'published'",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn list_blog_posts_admin(
    pool: &PgPool,
    offset: i64,
    limit: i64,
    status: Option<&PostStatus>,
    author_id: Option<Uuid>,
    search: Option<&str>,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let mut where_clauses = vec!["1=1".to_string()];
    if status.is_none() {
        where_clauses.push("status <> 'trash'".to_string());
    }
    if status.is_some() {
        where_clauses.push("status = $3".to_string());
    }
    if author_id.is_some() {
        where_clauses.push("author_id = $4".to_string());
    }
    if search.is_some() {
        where_clauses.push("(title ILIKE $5 OR content ILIKE $5)".to_string());
    }
    let where_clause = where_clauses.join(" AND ");

    let query_str = format!(
        "SELECT * FROM blog_posts WHERE {} ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
        where_clause
    );
    let count_str = format!("SELECT COUNT(*) FROM blog_posts WHERE {}", where_clause);

    let mut q = sqlx::query_as::<_, BlogPost>(&query_str)
        .bind(limit)
        .bind(offset);
    let mut cq = sqlx::query_as::<_, (i64,)>(&count_str);

    if let Some(s) = status {
        q = q.bind(s);
        cq = cq.bind(s);
    }
    if let Some(a) = author_id {
        q = q.bind(a);
        cq = cq.bind(a);
    }
    if let Some(s) = search {
        let pattern = format!("%{}%", s);
        q = q.bind(pattern.clone());
        cq = cq.bind(pattern);
    }

    let posts = q.fetch_all(pool).await?;
    let total = cq.fetch_one(pool).await?.0;
    Ok((posts, total))
}

pub async fn list_published_posts(
    pool: &PgPool,
    offset: i64,
    limit: i64,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let posts = sqlx::query_as::<_, BlogPost>(
        "SELECT * FROM blog_posts WHERE status = 'published' ORDER BY is_sticky DESC, published_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM blog_posts WHERE status = 'published'")
            .fetch_one(pool)
            .await?;
    Ok((posts, total.0))
}

pub async fn list_published_posts_by_category(
    pool: &PgPool,
    category_slug: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let posts = sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT bp.* FROM blog_posts bp
        JOIN blog_post_categories bpc ON bp.id = bpc.post_id
        JOIN blog_categories bc ON bpc.category_id = bc.id
        WHERE bp.status = 'published' AND bc.slug = $1
        ORDER BY bp.is_sticky DESC, bp.published_at DESC LIMIT $2 OFFSET $3
        "#,
    )
    .bind(category_slug)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM blog_posts bp
        JOIN blog_post_categories bpc ON bp.id = bpc.post_id
        JOIN blog_categories bc ON bpc.category_id = bc.id
        WHERE bp.status = 'published' AND bc.slug = $1
        "#,
    )
    .bind(category_slug)
    .fetch_one(pool)
    .await?;
    Ok((posts, total.0))
}

pub async fn list_published_posts_by_tag(
    pool: &PgPool,
    tag_slug: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let posts = sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT bp.* FROM blog_posts bp
        JOIN blog_post_tags bpt ON bp.id = bpt.post_id
        JOIN blog_tags bt ON bpt.tag_id = bt.id
        WHERE bp.status = 'published' AND bt.slug = $1
        ORDER BY bp.is_sticky DESC, bp.published_at DESC LIMIT $2 OFFSET $3
        "#,
    )
    .bind(tag_slug)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM blog_posts bp
        JOIN blog_post_tags bpt ON bp.id = bpt.post_id
        JOIN blog_tags bt ON bpt.tag_id = bt.id
        WHERE bp.status = 'published' AND bt.slug = $1
        "#,
    )
    .bind(tag_slug)
    .fetch_one(pool)
    .await?;
    Ok((posts, total.0))
}

pub async fn list_all_published_slugs(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT slug FROM blog_posts WHERE status = 'published' ORDER BY published_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

// ── Post ↔ Category / Tag junctions ───────────────────────────────────

pub async fn set_post_categories(
    pool: &PgPool,
    post_id: Uuid,
    category_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_post_categories WHERE post_id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;

    for cid in category_ids {
        sqlx::query("INSERT INTO blog_post_categories (post_id, category_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(post_id)
            .bind(cid)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn set_post_tags(
    pool: &PgPool,
    post_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_post_tags WHERE post_id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;

    for tid in tag_ids {
        sqlx::query(
            "INSERT INTO blog_post_tags (post_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(post_id)
        .bind(tid)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn get_categories_for_post(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Vec<BlogCategory>, sqlx::Error> {
    sqlx::query_as::<_, BlogCategory>(
        r#"
        SELECT bc.* FROM blog_categories bc
        JOIN blog_post_categories bpc ON bc.id = bpc.category_id
        WHERE bpc.post_id = $1 ORDER BY bc.sort_order, bc.name
        "#,
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

pub async fn get_tags_for_post(pool: &PgPool, post_id: Uuid) -> Result<Vec<BlogTag>, sqlx::Error> {
    sqlx::query_as::<_, BlogTag>(
        r#"
        SELECT bt.* FROM blog_tags bt
        JOIN blog_post_tags bpt ON bt.id = bpt.tag_id
        WHERE bpt.post_id = $1 ORDER BY bt.name
        "#,
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

// ── Blog Categories ───────────────────────────────────────────────────

pub async fn create_blog_category(
    pool: &PgPool,
    req: &CreateCategoryRequest,
) -> Result<BlogCategory, sqlx::Error> {
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.name));

    sqlx::query_as::<_, BlogCategory>(
        r#"
        INSERT INTO blog_categories (id, name, slug, description, parent_id, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&req.name)
    .bind(&slug)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.parent_id)
    .bind(req.sort_order.unwrap_or(0))
    .fetch_one(pool)
    .await
}

pub async fn update_blog_category(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateCategoryRequest,
) -> Result<BlogCategory, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogCategory>("SELECT * FROM blog_categories WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    let name = req.name.as_deref().unwrap_or(&existing.name);
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| existing.slug.clone());

    sqlx::query_as::<_, BlogCategory>(
        r#"
        UPDATE blog_categories SET name = $1, slug = $2, description = $3, parent_id = $4, sort_order = $5
        WHERE id = $6 RETURNING *
        "#,
    )
    .bind(name)
    .bind(&slug)
    .bind(req.description.as_deref().unwrap_or(existing.description.as_deref().unwrap_or("")))
    .bind(req.parent_id.or(existing.parent_id))
    .bind(req.sort_order.unwrap_or(existing.sort_order))
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_blog_category(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_categories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_blog_categories(pool: &PgPool) -> Result<Vec<BlogCategory>, sqlx::Error> {
    sqlx::query_as::<_, BlogCategory>("SELECT * FROM blog_categories ORDER BY sort_order, name")
        .fetch_all(pool)
        .await
}

// ── Blog Tags ─────────────────────────────────────────────────────────

pub async fn create_blog_tag(
    pool: &PgPool,
    req: &CreateTagRequest,
) -> Result<BlogTag, sqlx::Error> {
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.name));

    sqlx::query_as::<_, BlogTag>(
        "INSERT INTO blog_tags (id, name, slug) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&req.name)
    .bind(&slug)
    .fetch_one(pool)
    .await
}

pub async fn delete_blog_tag(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_tags WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_blog_tags(pool: &PgPool) -> Result<Vec<BlogTag>, sqlx::Error> {
    sqlx::query_as::<_, BlogTag>("SELECT * FROM blog_tags ORDER BY name")
        .fetch_all(pool)
        .await
}

// ── Blog Revisions ────────────────────────────────────────────────────

pub async fn create_blog_revision(
    pool: &PgPool,
    post_id: Uuid,
    author_id: Uuid,
    title: &str,
    content: &str,
    content_json: Option<&serde_json::Value>,
) -> Result<BlogRevision, sqlx::Error> {
    let next_rev: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM blog_revisions WHERE post_id = $1",
    )
    .bind(post_id)
    .fetch_one(pool)
    .await?;

    sqlx::query_as::<_, BlogRevision>(
        r#"
        INSERT INTO blog_revisions (id, post_id, author_id, title, content, content_json, revision_number)
        VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(post_id)
    .bind(author_id)
    .bind(title)
    .bind(content)
    .bind(content_json)
    .bind(next_rev.0 as i32)
    .fetch_one(pool)
    .await
}

pub async fn list_blog_revisions(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Vec<BlogRevision>, sqlx::Error> {
    sqlx::query_as::<_, BlogRevision>(
        "SELECT * FROM blog_revisions WHERE post_id = $1 ORDER BY revision_number DESC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

pub async fn get_blog_revision(
    pool: &PgPool,
    revision_id: Uuid,
) -> Result<Option<BlogRevision>, sqlx::Error> {
    sqlx::query_as::<_, BlogRevision>("SELECT * FROM blog_revisions WHERE id = $1")
        .bind(revision_id)
        .fetch_optional(pool)
        .await
}

// ── Media ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn create_media(
    pool: &PgPool,
    uploader_id: Uuid,
    filename: &str,
    original_filename: &str,
    title: Option<&str>,
    mime_type: &str,
    file_size: i64,
    width: Option<i32>,
    height: Option<i32>,
    storage_path: &str,
    url: &str,
) -> Result<Media, sqlx::Error> {
    sqlx::query_as::<_, Media>(
        r#"
        INSERT INTO media (id, uploader_id, filename, original_filename, title, mime_type, file_size, width, height, storage_path, url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(uploader_id)
    .bind(filename)
    .bind(original_filename)
    .bind(title)
    .bind(mime_type)
    .bind(file_size)
    .bind(width)
    .bind(height)
    .bind(storage_path)
    .bind(url)
    .fetch_one(pool)
    .await
}

pub async fn update_media(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateMediaRequest,
) -> Result<Media, sqlx::Error> {
    let existing = sqlx::query_as::<_, Media>("SELECT * FROM media WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    sqlx::query_as::<_, Media>(
        "UPDATE media SET title = $1, alt_text = $2, caption = $3, focal_x = $4, focal_y = $5 WHERE id = $6 RETURNING *",
    )
    .bind(req.title.as_deref().or(existing.title.as_deref()))
    .bind(req.alt_text.as_deref().or(existing.alt_text.as_deref()))
    .bind(req.caption.as_deref().or(existing.caption.as_deref()))
    .bind(req.focal_x.unwrap_or(existing.focal_x))
    .bind(req.focal_y.unwrap_or(existing.focal_y))
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_media(pool: &PgPool, id: Uuid) -> Result<Option<Media>, sqlx::Error> {
    sqlx::query_as::<_, Media>("DELETE FROM media WHERE id = $1 RETURNING *")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_media(pool: &PgPool, id: Uuid) -> Result<Option<Media>, sqlx::Error> {
    sqlx::query_as::<_, Media>("SELECT * FROM media WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_media(
    pool: &PgPool,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Media>, i64), sqlx::Error> {
    let items = sqlx::query_as::<_, Media>(
        "SELECT * FROM media ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM media")
        .fetch_one(pool)
        .await?;
    Ok((items, total.0))
}

// ── Post Meta ──────────────────────────────────────────────────────────────

pub async fn list_post_meta(pool: &PgPool, post_id: Uuid) -> Result<Vec<PostMeta>, sqlx::Error> {
    sqlx::query_as::<_, PostMeta>(
        "SELECT * FROM post_meta WHERE post_id = $1 ORDER BY meta_key ASC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

pub async fn upsert_post_meta(
    pool: &PgPool,
    post_id: Uuid,
    meta_key: &str,
    meta_value: &str,
) -> Result<PostMeta, sqlx::Error> {
    sqlx::query_as::<_, PostMeta>(
        r#"
        INSERT INTO post_meta (id, post_id, meta_key, meta_value)
        VALUES (gen_random_uuid(), $1, $2, $3)
        ON CONFLICT (post_id, meta_key)
        DO UPDATE SET meta_value = EXCLUDED.meta_value, updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(post_id)
    .bind(meta_key)
    .bind(meta_value)
    .fetch_one(pool)
    .await
}

pub async fn delete_post_meta(
    pool: &PgPool,
    post_id: Uuid,
    meta_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM post_meta WHERE post_id = $1 AND meta_key = $2")
        .bind(post_id)
        .bind(meta_key)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Analytics ─────────────────────────────────────────────────────────────

pub async fn ingest_analytics_events(
    pool: &PgPool,
    session_id: Uuid,
    user_id: Option<Uuid>,
    events: Vec<(String, String, Option<String>, serde_json::Value)>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO analytics_sessions (id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (id) DO UPDATE SET
            user_id = COALESCE(EXCLUDED.user_id, analytics_sessions.user_id),
            updated_at = NOW()
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    for (event_type, path, referrer, metadata) in events {
        sqlx::query(
            r#"
            INSERT INTO analytics_events (session_id, event_type, path, referrer, metadata)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(session_id)
        .bind(&event_type)
        .bind(&path)
        .bind(referrer.as_deref())
        .bind(&metadata)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct AnalyticsDayRow {
    pub day: NaiveDate,
    pub page_views: i64,
    pub unique_sessions: i64,
    pub impressions: i64,
}

pub async fn analytics_time_series(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<Vec<AnalyticsDayRow>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsDayRow>(
        r#"
        SELECT
            (date_trunc('day', created_at AT TIME ZONE 'UTC'))::date AS day,
            SUM(CASE WHEN event_type = 'page_view' THEN 1 ELSE 0 END)::bigint AS page_views,
            COUNT(DISTINCT CASE WHEN event_type = 'page_view' THEN session_id END)::bigint AS unique_sessions,
            SUM(CASE WHEN event_type = 'impression' THEN 1 ELSE 0 END)::bigint AS impressions
        FROM analytics_events
        WHERE created_at >= $1 AND created_at < $2
        GROUP BY 1
        ORDER BY 1 ASC
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct AnalyticsTopPageRow {
    pub path: String,
    pub views: i64,
    pub sessions: i64,
}

pub async fn analytics_top_pages(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<AnalyticsTopPageRow>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsTopPageRow>(
        r#"
        SELECT
            path,
            COUNT(*)::bigint AS views,
            COUNT(DISTINCT session_id)::bigint AS sessions
        FROM analytics_events
        WHERE event_type = 'page_view' AND created_at >= $1 AND created_at < $2
        GROUP BY path
        ORDER BY views DESC
        LIMIT $3
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Sessions with at least one page_view in the window: "bounce" = exactly one page_view.
pub async fn analytics_bounce_stats(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<(i64, i64), sqlx::Error> {
    let row: (i64, i64) = sqlx::query_as(
        r#"
        WITH session_pv AS (
            SELECT session_id, COUNT(*)::bigint AS pv
            FROM analytics_events
            WHERE event_type = 'page_view' AND created_at >= $1 AND created_at < $2
            GROUP BY session_id
        )
        SELECT
            COUNT(*) FILTER (WHERE pv = 1)::bigint AS bounced,
            COUNT(*)::bigint AS eligible
        FROM session_pv
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

#[derive(Debug, sqlx::FromRow)]
pub struct DailyRevenueRow {
    pub day: NaiveDate,
    pub revenue_cents: i64,
}

pub async fn analytics_sales_revenue_daily(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<Vec<DailyRevenueRow>, sqlx::Error> {
    sqlx::query_as::<_, DailyRevenueRow>(
        r#"
        SELECT
            (date_trunc('day', created_at AT TIME ZONE 'UTC'))::date AS day,
            COALESCE(SUM(amount_cents), 0)::bigint AS revenue_cents
        FROM sales_events
        WHERE created_at >= $1 AND created_at < $2
        GROUP BY 1
        ORDER BY 1 ASC
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_all(pool)
    .await
}

pub async fn analytics_sales_revenue_total_cents(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(amount_cents), 0)::bigint
        FROM sales_events
        WHERE created_at >= $1 AND created_at < $2
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

#[derive(Debug, sqlx::FromRow)]
pub struct RecentSaleRow {
    pub id: Uuid,
    pub event_type: String,
    pub amount_cents: i32,
    pub user_email: String,
    pub created_at: DateTime<Utc>,
}

pub async fn analytics_recent_sales(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<RecentSaleRow>, sqlx::Error> {
    sqlx::query_as::<_, RecentSaleRow>(
        r#"
        SELECT se.id, se.event_type, se.amount_cents, u.email AS user_email, se.created_at
        FROM sales_events se
        JOIN users u ON u.id = se.user_id
        WHERE se.created_at >= $1 AND se.created_at < $2
        ORDER BY se.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Active monthly/annual plan prices from `pricing_plans` (slugs `monthly` / `annual`).
pub async fn pricing_monthly_annual_cents(pool: &PgPool) -> Result<(i32, i32), sqlx::Error> {
    let row: (Option<i32>, Option<i32>) = sqlx::query_as(
        r#"
        SELECT
            MAX(amount_cents) FILTER (WHERE slug = 'monthly'),
            MAX(amount_cents) FILTER (WHERE slug = 'annual')
        FROM pricing_plans
        WHERE is_active = TRUE
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok((row.0.unwrap_or(0), row.1.unwrap_or(0)))
}

/// Estimated MRR / ARR from active subscription counts × public plan prices.
pub async fn admin_estimated_mrr_arr_cents(pool: &PgPool) -> Result<(i64, i64, i64), sqlx::Error> {
    let (monthly_price, annual_price) = pricing_monthly_annual_cents(pool).await?;
    let n_m = count_subscriptions_by_plan(pool, &SubscriptionPlan::Monthly).await?;
    let n_a = count_subscriptions_by_plan(pool, &SubscriptionPlan::Annual).await?;
    let mrr = n_m * monthly_price as i64 + (n_a * annual_price as i64) / 12;
    let arr = mrr * 12;
    let active = count_active_subscriptions(pool).await?;
    Ok((mrr, arr, active))
}

#[derive(Debug, sqlx::FromRow)]
pub struct AnalyticsCtrRow {
    pub day: NaiveDate,
    pub cta_id: String,
    pub impressions: i64,
    pub clicks: i64,
}

pub async fn analytics_ctr_breakdown(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<Vec<AnalyticsCtrRow>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsCtrRow>(
        r#"
        SELECT
            (date_trunc('day', created_at AT TIME ZONE 'UTC'))::date AS day,
            COALESCE(metadata->>'cta_id', '') AS cta_id,
            SUM(CASE WHEN event_type = 'impression' THEN 1 ELSE 0 END)::bigint AS impressions,
            SUM(CASE WHEN event_type = 'click' THEN 1 ELSE 0 END)::bigint AS clicks
        FROM analytics_events
        WHERE created_at >= $1 AND created_at < $2
          AND event_type IN ('impression', 'click')
          AND COALESCE(metadata->>'cta_id', '') <> ''
        GROUP BY 1, 2
        ORDER BY 1 ASC, 2 ASC
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_all(pool)
    .await
}

pub async fn analytics_totals(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<(i64, i64, i64), sqlx::Error> {
    let row: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE event_type = 'page_view')::bigint,
            COUNT(DISTINCT session_id) FILTER (WHERE event_type = 'page_view')::bigint,
            COUNT(*) FILTER (WHERE event_type = 'impression')::bigint
        FROM analytics_events
        WHERE created_at >= $1 AND created_at < $2
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

// ── Coupons (admin dashboard aggregate) ───────────────────────────────────

/// Single round-trip aggregation for the admin coupons dashboard. Counts the
/// `coupons` table by lifecycle bucket (`active`, `expired`, `scheduled`)
/// and sums `coupon_usages` for the global redemption + dollar-value totals.
/// Returns zeros when no rows exist so the handler never has to special-case
/// an empty database.
pub async fn coupon_stats(pool: &PgPool) -> Result<CouponStats, sqlx::Error> {
    let row: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM coupons)::bigint                      AS total,
            (SELECT COUNT(*) FROM coupons
                WHERE is_active = TRUE
                  AND (starts_at IS NULL OR starts_at <= NOW())
                  AND (expires_at IS NULL OR expires_at >  NOW()))::bigint AS active,
            (SELECT COUNT(*) FROM coupons
                WHERE expires_at IS NOT NULL AND expires_at < NOW())::bigint AS expired,
            (SELECT COUNT(*) FROM coupons
                WHERE starts_at IS NOT NULL AND starts_at > NOW())::bigint AS scheduled,
            (SELECT COUNT(*) FROM coupon_usages)::bigint                AS redemption_count,
            (SELECT COALESCE(SUM(discount_applied_cents), 0)
                FROM coupon_usages)::bigint                              AS total_discount_cents
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(CouponStats {
        total: row.0,
        active: row.1,
        expired: row.2,
        scheduled: row.3,
        redemption_count: row.4,
        total_discount_cents: row.5,
    })
}

// ── Popups (collection-level analytics) ───────────────────────────────────

/// Per-popup roll-up of impression / close / submit counts and the
/// derived conversion rate (`submits / impressions`, 0 when no impressions).
///
/// Single GROUP BY query so the admin index can render the whole list in
/// one round-trip; ordered by `submits DESC, impressions DESC` so the
/// best-performing popups float to the top.
pub async fn list_popup_analytics_summaries(
    pool: &PgPool,
) -> Result<Vec<PopupAnalyticsSummary>, sqlx::Error> {
    sqlx::query_as::<_, PopupAnalyticsSummary>(
        r#"
        SELECT
            p.id              AS popup_id,
            p.name            AS popup_name,
            p.popup_type      AS popup_type,
            p.is_active       AS is_active,
            COALESCE(SUM(CASE WHEN e.event_type = 'impression' THEN 1 ELSE 0 END), 0)::bigint AS impressions,
            COALESCE(SUM(CASE WHEN e.event_type = 'close'      THEN 1 ELSE 0 END), 0)::bigint AS closes,
            COALESCE(SUM(CASE WHEN e.event_type = 'submit'     THEN 1 ELSE 0 END), 0)::bigint AS submits,
            CASE
                WHEN COALESCE(SUM(CASE WHEN e.event_type = 'impression' THEN 1 ELSE 0 END), 0) > 0
                THEN COALESCE(SUM(CASE WHEN e.event_type = 'submit' THEN 1 ELSE 0 END), 0)::float8
                     / NULLIF(SUM(CASE WHEN e.event_type = 'impression' THEN 1 ELSE 0 END), 0)::float8
                ELSE 0.0
            END AS conversion_rate
        FROM popups p
        LEFT JOIN popup_events e ON e.popup_id = p.id
        GROUP BY p.id, p.name, p.popup_type, p.is_active
        ORDER BY submits DESC, impressions DESC
        "#,
    )
    .fetch_all(pool)
    .await
}
