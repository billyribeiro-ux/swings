//! POP-05 repo helpers for `popup_visitor_state`.
//!
//! Functions are thin wrappers over sqlx so handlers can express
//! "record an impression" / "record a dismissal" / "record a conversion"
//! at the call site without mixing raw SQL into request-handling code.
//! Every write is an UPSERT so the first call for a new `(anonymous_id,
//! popup_id)` pair inserts the row and every subsequent call mutates it.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::frequency::VisitorState;
use crate::error::AppResult;

pub async fn load_visitor_state(
    pool: &sqlx::PgPool,
    anonymous_id: Uuid,
    popup_id: Uuid,
) -> AppResult<Option<VisitorState>> {
    let row = sqlx::query_as::<_, VisitorState>(
        r#"
        SELECT anonymous_id, popup_id, first_shown_at, last_shown_at,
               times_shown, times_dismissed, converted, updated_at
        FROM popup_visitor_state
        WHERE anonymous_id = $1 AND popup_id = $2
        "#,
    )
    .bind(anonymous_id)
    .bind(popup_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn record_impression(
    pool: &sqlx::PgPool,
    anonymous_id: Uuid,
    popup_id: Uuid,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO popup_visitor_state
            (anonymous_id, popup_id, first_shown_at, last_shown_at,
             times_shown, times_dismissed, converted, updated_at)
        VALUES ($1, $2, $3, $3, 1, 0, FALSE, $3)
        ON CONFLICT (anonymous_id, popup_id) DO UPDATE SET
            last_shown_at = EXCLUDED.last_shown_at,
            times_shown   = popup_visitor_state.times_shown + 1,
            first_shown_at = COALESCE(popup_visitor_state.first_shown_at,
                                       EXCLUDED.first_shown_at),
            updated_at    = EXCLUDED.updated_at
        "#,
    )
    .bind(anonymous_id)
    .bind(popup_id)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_dismissal(
    pool: &sqlx::PgPool,
    anonymous_id: Uuid,
    popup_id: Uuid,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO popup_visitor_state
            (anonymous_id, popup_id, times_shown, times_dismissed, converted, updated_at)
        VALUES ($1, $2, 0, 1, FALSE, $3)
        ON CONFLICT (anonymous_id, popup_id) DO UPDATE SET
            times_dismissed = popup_visitor_state.times_dismissed + 1,
            updated_at      = EXCLUDED.updated_at
        "#,
    )
    .bind(anonymous_id)
    .bind(popup_id)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn record_conversion(
    pool: &sqlx::PgPool,
    anonymous_id: Uuid,
    popup_id: Uuid,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO popup_visitor_state
            (anonymous_id, popup_id, times_shown, times_dismissed, converted, updated_at)
        VALUES ($1, $2, 0, 0, TRUE, $3)
        ON CONFLICT (anonymous_id, popup_id) DO UPDATE SET
            converted  = TRUE,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(anonymous_id)
    .bind(popup_id)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
