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

pub async fn seed_admin(pool: &PgPool, email: &str, password: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hash error: {e}"))?
        .to_string();

    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, role)
        VALUES ($1, $2, $3, $4, 'admin')
        ON CONFLICT (email) DO UPDATE
            SET password_hash = EXCLUDED.password_hash,
                name = EXCLUDED.name,
                updated_at = NOW()
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(&password_hash)
    .bind(name)
    .execute(pool)
    .await?;

    tracing::info!("Admin user upserted: {}", email);
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

#[allow(dead_code)]
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
            if c == '<' { (out, true) }
            else if c == '>' { out.push(' '); (out, false) }
            else if !in_tag { out.push(c); (out, false) }
            else { (out, true) }
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
    let slug = req.slug.as_deref()
        .map(|s| slugify(s))
        .unwrap_or_else(|| slugify(&req.title));
    let content = req.content.as_deref().unwrap_or("");
    let wc = compute_word_count(content);
    let rt = compute_reading_time(wc);
    let status = req.status.clone().unwrap_or(PostStatus::Draft);
    let published_at = if status == PostStatus::Published { Some(Utc::now()) } else { None };

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
    let slug = req.slug.as_deref()
        .map(|s| slugify(s))
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
            author_id = $21, updated_at = NOW()
        WHERE id = $22 RETURNING *
        "#,
    )
    .bind(title)
    .bind(&slug)
    .bind(content)
    .bind(req.content_json.as_ref().or(existing.content_json.as_ref()))
    .bind(req.excerpt.as_deref().unwrap_or(existing.excerpt.as_deref().unwrap_or("")))
    .bind(req.featured_image_id.or(existing.featured_image_id))
    .bind(&status)
    .bind(req.visibility.as_deref().unwrap_or(&existing.visibility))
    .bind(new_password_hash.as_deref())
    .bind(req.format.as_deref().unwrap_or(&existing.format))
    .bind(req.is_sticky.unwrap_or(existing.is_sticky))
    .bind(req.allow_comments.unwrap_or(existing.allow_comments))
    .bind(req.meta_title.as_deref().unwrap_or(existing.meta_title.as_deref().unwrap_or("")))
    .bind(req.meta_description.as_deref().unwrap_or(existing.meta_description.as_deref().unwrap_or("")))
    .bind(req.canonical_url.as_deref().unwrap_or(existing.canonical_url.as_deref().unwrap_or("")))
    .bind(req.og_image_url.as_deref().unwrap_or(existing.og_image_url.as_deref().unwrap_or("")))
    .bind(rt)
    .bind(wc)
    .bind(req.scheduled_at.or(existing.scheduled_at))
    .bind(published_at)
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

pub async fn update_post_status(
    pool: &PgPool,
    post_id: Uuid,
    status: &PostStatus,
) -> Result<BlogPost, sqlx::Error> {
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

pub async fn get_blog_post_by_slug(pool: &PgPool, slug: &str) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        "SELECT * FROM blog_posts WHERE slug = $1 AND status = 'published'"
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
    if status.is_some() { where_clauses.push("status = $3".to_string()); }
    if author_id.is_some() { where_clauses.push("author_id = $4".to_string()); }
    if search.is_some() { where_clauses.push("(title ILIKE $5 OR content ILIKE $5)".to_string()); }
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

    if let Some(s) = status { q = q.bind(s); cq = cq.bind(s); }
    if let Some(a) = author_id { q = q.bind(a); cq = cq.bind(a); }
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

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM blog_posts WHERE status = 'published'")
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
        "SELECT slug FROM blog_posts WHERE status = 'published' ORDER BY published_at DESC"
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
        sqlx::query("INSERT INTO blog_post_tags (post_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
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

pub async fn get_tags_for_post(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Vec<BlogTag>, sqlx::Error> {
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
    let slug = req.slug.as_deref()
        .map(|s| slugify(s))
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
    let slug = req.slug.as_deref()
        .map(|s| slugify(s))
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
    sqlx::query_as::<_, BlogCategory>(
        "SELECT * FROM blog_categories ORDER BY sort_order, name"
    )
    .fetch_all(pool)
    .await
}

// ── Blog Tags ─────────────────────────────────────────────────────────

pub async fn create_blog_tag(
    pool: &PgPool,
    req: &CreateTagRequest,
) -> Result<BlogTag, sqlx::Error> {
    let slug = req.slug.as_deref()
        .map(|s| slugify(s))
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
        "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM blog_revisions WHERE post_id = $1"
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
        "SELECT * FROM blog_revisions WHERE post_id = $1 ORDER BY revision_number DESC"
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
        "SELECT * FROM media ORDER BY created_at DESC LIMIT $1 OFFSET $2"
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
