use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;

// ── Users ───────────────────────────────────────────────────────────────

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    name: &str,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, password_hash, name, role)
        VALUES ($1, $2, $3, $4, 'member')
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(password_hash)
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
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

pub async fn recent_members(pool: &PgPool, limit: i64) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE role = 'member' ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

// ── Refresh Tokens ──────────────────────────────────────────────────────

pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
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

pub async fn delete_refresh_token(pool: &PgPool, token_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_user_refresh_tokens(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Subscriptions ───────────────────────────────────────────────────────

pub async fn upsert_subscription(
    pool: &PgPool,
    user_id: Uuid,
    stripe_customer_id: &str,
    stripe_subscription_id: &str,
    plan: &SubscriptionPlan,
    status: &SubscriptionStatus,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Subscription, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        r#"
        INSERT INTO subscriptions (id, user_id, stripe_customer_id, stripe_subscription_id, plan, status, current_period_start, current_period_end)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (stripe_subscription_id)
        DO UPDATE SET status = $6, plan = $5, current_period_start = $7, current_period_end = $8, updated_at = NOW()
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
    .fetch_one(pool)
    .await
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
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE status = 'active'")
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

pub async fn count_subscriptions_by_plan(
    pool: &PgPool,
    plan: &SubscriptionPlan,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM subscriptions WHERE plan = $1 AND status = 'active'",
    )
    .bind(plan)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
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

        let t: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM watchlists WHERE published = true")
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
    .bind(req.invalidation.as_deref().unwrap_or(&existing.invalidation))
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

pub async fn enroll_user(
    pool: &PgPool,
    user_id: Uuid,
    course_id: &str,
) -> Result<CourseEnrollment, sqlx::Error> {
    sqlx::query_as::<_, CourseEnrollment>(
        r#"
        INSERT INTO course_enrollments (id, user_id, course_id) VALUES ($1, $2, $3)
        ON CONFLICT (user_id, course_id) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(course_id)
    .fetch_one(pool)
    .await
}

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
