//! EC-10: Memberships — plans, grants, restriction engine, drip rules,
//! member-only discounts.
//!
//! The restriction engine is the hot path: every protected handler asks
//! [`can_access`] whether the caller may see a specific resource. The
//! decision is built from the union of every active membership's
//! `grants_access_to` envelope (categories / products / urls / courses).
//!
//! Drip materialisation runs asynchronously: a worker walks the plan's
//! `drip_rules` and bumps `grants_access_to` on the per-user membership
//! row at each `after_days` checkpoint.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MembershipPlan {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub grants_access_to: serde_json::Value,
    pub drip_rules: serde_json::Value,
    pub default_duration_days: Option<i32>,
    pub price_cents: Option<i64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Membership {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub granted_by: String,
    pub status: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Resource the access engine resolves against. The string variants are
/// chosen to match the JSON keys in `grants_access_to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Category(Uuid),
    Product(Uuid),
    Course(Uuid),
    Url(String),
}

impl Resource {
    fn key(&self) -> &'static str {
        match self {
            Resource::Category(_) => "categories",
            Resource::Product(_) => "products",
            Resource::Course(_) => "courses",
            Resource::Url(_) => "urls",
        }
    }
}

/// Pure access check — given the per-plan `grants_access_to` blob and a
/// resource, decide whether the resource is granted.
pub fn plan_grants(grants: &serde_json::Value, resource: &Resource) -> bool {
    let Some(arr) = grants.get(resource.key()).and_then(|v| v.as_array()) else {
        return false;
    };
    match resource {
        Resource::Category(id) | Resource::Product(id) | Resource::Course(id) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .filter_map(|s| Uuid::parse_str(s).ok())
            .any(|x| &x == id),
        Resource::Url(path) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .any(|pat| url_pattern_matches(pat, path)),
    }
}

/// Trailing-`*` glob match for URL grants. Plans declare patterns like
/// `/courses/intro/*` to grant the entire prefix.
fn url_pattern_matches(pattern: &str, path: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/*") {
        path == prefix || path.starts_with(&format!("{prefix}/"))
    } else {
        pattern == path
    }
}

/// Run `plan_grants` over every active membership the user holds.
/// `now` is parameterised so tests can advance time without mocking the clock.
pub async fn can_access(
    pool: &PgPool,
    user_id: Uuid,
    resource: &Resource,
    now: DateTime<Utc>,
) -> AppResult<bool> {
    let rows = sqlx::query_as::<_, (serde_json::Value,)>(
        r#"
        SELECT mp.grants_access_to
        FROM memberships m
        JOIN membership_plans mp ON mp.id = m.plan_id
        WHERE m.user_id = $1
          AND m.status = 'active'
          AND m.starts_at <= $2
          AND (m.ends_at IS NULL OR m.ends_at > $2)
        "#,
    )
    .bind(user_id)
    .bind(now)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().any(|(g,)| plan_grants(g, resource)))
}

// ── Repository ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn create_plan(
    pool: &PgPool,
    slug: &str,
    name: &str,
    description: &str,
    grants_access_to: &serde_json::Value,
    drip_rules: &serde_json::Value,
    default_duration_days: Option<i32>,
    price_cents: Option<i64>,
) -> AppResult<MembershipPlan> {
    let row = sqlx::query_as::<_, MembershipPlan>(
        r#"
        INSERT INTO membership_plans
            (slug, name, description, grants_access_to, drip_rules,
             default_duration_days, price_cents)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, slug, name, description, grants_access_to, drip_rules,
                  default_duration_days, price_cents, is_active,
                  created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(grants_access_to)
    .bind(drip_rules)
    .bind(default_duration_days)
    .bind(price_cents)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn grant_membership(
    pool: &PgPool,
    user_id: Uuid,
    plan_id: Uuid,
    granted_by: &str,
    duration_days: Option<i32>,
) -> AppResult<Membership> {
    let ends_at = duration_days.map(|d| Utc::now() + Duration::days(d as i64));
    let row = sqlx::query_as::<_, Membership>(
        r#"
        INSERT INTO memberships (user_id, plan_id, granted_by, ends_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, plan_id, granted_by, status,
                  starts_at, ends_at, metadata, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(plan_id)
    .bind(granted_by)
    .bind(ends_at)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn list_user_memberships(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<Membership>> {
    let rows = sqlx::query_as::<_, Membership>(
        r#"
        SELECT id, user_id, plan_id, granted_by, status,
               starts_at, ends_at, metadata, created_at, updated_at
        FROM memberships WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn cancel_membership(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE memberships
           SET status = 'cancelled',
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plan_grants_category_by_uuid() {
        let id = Uuid::new_v4();
        let g = json!({ "categories": [id.to_string()] });
        assert!(plan_grants(&g, &Resource::Category(id)));
        assert!(!plan_grants(&g, &Resource::Category(Uuid::new_v4())));
    }

    #[test]
    fn plan_grants_url_with_glob() {
        let g = json!({ "urls": ["/courses/intro/*"] });
        assert!(plan_grants(&g, &Resource::Url("/courses/intro".into())));
        assert!(plan_grants(
            &g,
            &Resource::Url("/courses/intro/lesson-1".into())
        ));
        assert!(!plan_grants(&g, &Resource::Url("/courses/advanced".into())));
    }

    #[test]
    fn plan_grants_exact_url_when_no_glob() {
        let g = json!({ "urls": ["/about"] });
        assert!(plan_grants(&g, &Resource::Url("/about".into())));
        assert!(!plan_grants(&g, &Resource::Url("/about/team".into())));
    }

    #[test]
    fn plan_grants_returns_false_on_missing_key() {
        let g = json!({});
        assert!(!plan_grants(&g, &Resource::Category(Uuid::new_v4())));
        assert!(!plan_grants(&g, &Resource::Url("/x".into())));
    }

    #[test]
    fn plan_grants_handles_invalid_uuid_strings_gracefully() {
        let id = Uuid::new_v4();
        let g = json!({ "categories": ["not-a-uuid", id.to_string()] });
        assert!(plan_grants(&g, &Resource::Category(id)));
    }
}
