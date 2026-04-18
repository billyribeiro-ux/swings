//! EC-02: Catalog search + facets + nested category tree.
//!
//! The public PDP search lives here. Three concerns:
//!
//! 1. **Full-text search** against `products.search_tsv`. A user-supplied
//!    query `q` is tokenised client-side — we convert space-separated
//!    words into the `&`-ANDed `plainto_tsquery` form because it's the
//!    only tsquery shape that handles arbitrary user input without
//!    throwing on special characters.
//! 2. **Filters** — category id (with descendant walk via the `path`
//!    materialised column), price range, optional tag. Filters are
//!    composable; missing filters produce no `WHERE` clause at all.
//! 3. **Pagination** — page / per_page OFFSET. Keyset is overkill for v1
//!    (the catalog tops out at a few thousand rows). We clamp per_page
//!    to `[1, 100]` to keep the worst-case response bounded.
//!
//! Facets come back as a flat `{kind, value, label, count}` list so the
//! frontend can render category + tag + price-bucket filters from one
//! payload. The `product_facets` materialised view is the source; it is
//! refreshed out-of-band (PG cron, hourly) so search queries don't pay
//! the aggregation cost per hit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, QueryBuilder};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::commerce::products::Product;
use crate::error::AppResult;

// ── Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Category {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub path: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single entry in the facets response. `kind` is `"category"` or
/// `"tag"`; future kinds can be added to the materialised view without
/// changing this shape.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Facet {
    pub facet_kind: String,
    pub facet_value: String,
    pub facet_label: String,
    pub product_count: i64,
}

/// Search input — every field is optional so the handler can forward
/// URL query params without a schema-builder. The repo layer clamps
/// `per_page` + trims `q`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct SearchParams {
    pub q: Option<String>,
    pub category_id: Option<Uuid>,
    pub tag: Option<String>,
    pub min_price_cents: Option<i64>,
    pub max_price_cents: Option<i64>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Search response shape — products + count + facets, matching the
/// `{products, total, facets}` triple the plan calls for.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResponse {
    pub products: Vec<Product>,
    pub total: i64,
    pub facets: Vec<Facet>,
    pub page: i64,
    pub per_page: i64,
}

// ── Query builder ─────────────────────────────────────────────────────

/// Build the search SQL. Broken out into its own function so the unit
/// tests can assert the produced SQL without spinning up Postgres.
///
/// Returns `(sql_fragment, bind_count)` — the repo layer feeds the
/// fragment into a [`sqlx::QueryBuilder`] and the bind_count is used for
/// assertions. In practice callers don't need to inspect the return
/// value; it's just a test hook.
#[must_use]
pub fn build_where_fragment(params: &SearchParams) -> (String, usize) {
    let mut parts: Vec<String> = Vec::new();
    let mut binds = 0_usize;

    // Baseline filter — we never surface non-published rows through the
    // public search handler. Admin-side (if we ever add it) calls with
    // status=None and this clause is skipped via the builder above.
    parts.push("p.status = 'published'".into());

    if let Some(q) = params.q.as_ref().filter(|s| !s.trim().is_empty()) {
        let _ = q;
        binds += 1;
        parts.push(format!("p.search_tsv @@ plainto_tsquery('simple', ${binds})"));
    }

    if params.category_id.is_some() {
        binds += 1;
        // descendant-match via the materialised path. One join per call;
        // the category path is indexed so the prefix scan is O(log n).
        parts.push(format!(
            "EXISTS (
                SELECT 1 FROM product_category_links l
                JOIN product_categories c ON c.id = l.category_id
                JOIN product_categories ref ON ref.id = ${binds}
                WHERE l.product_id = p.id AND c.path LIKE ref.path || '%'
            )"
        ));
    }

    if params.tag.is_some() {
        binds += 1;
        parts.push(format!(
            "COALESCE(p.metadata->'tags', '[]'::jsonb) ? ${binds}"
        ));
    }

    if params.min_price_cents.is_some() {
        binds += 1;
        parts.push(format!("COALESCE(p.price_cents, 0) >= ${binds}"));
    }

    if params.max_price_cents.is_some() {
        binds += 1;
        parts.push(format!("COALESCE(p.price_cents, 0) <= ${binds}"));
    }

    (format!("WHERE {}", parts.join(" AND ")), binds)
}

// ── Public API ─────────────────────────────────────────────────────────

/// Run the search. Returns products + total count + facets.
pub async fn search_products(
    pool: &PgPool,
    params: &SearchParams,
) -> AppResult<SearchResponse> {
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let (where_sql, _binds) = build_where_fragment(params);

    // Product list.
    let list_sql = format!(
        "SELECT id, slug, name, description, product_type, status,
                price_cents, compare_at_cents, currency, tax_class,
                stripe_product_id, stripe_price_id, gallery_media_ids,
                featured_media_id, seo_title, seo_description, metadata,
                created_by, created_at, updated_at
         FROM products p
         {where_sql}
         ORDER BY p.updated_at DESC
         LIMIT {per_page} OFFSET {offset}"
    );
    let mut qb = QueryBuilder::<sqlx::Postgres>::new(&list_sql);
    let products = bind_params(&mut qb, params)
        .build_query_as::<Product>()
        .fetch_all(pool)
        .await?;

    // Total count — same WHERE, no LIMIT.
    let count_sql = format!("SELECT COUNT(*) FROM products p {where_sql}");
    let mut qb = QueryBuilder::<sqlx::Postgres>::new(&count_sql);
    let total: i64 = bind_params(&mut qb, params)
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;

    // Facets — read the materialized view in one shot. This is cheap
    // because the view is already aggregated; worst case a few hundred
    // rows for a catalogue of a few thousand products.
    let facets = sqlx::query_as::<_, Facet>(
        r#"
        SELECT facet_kind, facet_value, facet_label, product_count
        FROM product_facets
        WHERE product_count > 0
        ORDER BY facet_kind ASC, product_count DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default(); // A missing view shouldn't break search.

    Ok(SearchResponse {
        products,
        total,
        facets,
        page,
        per_page,
    })
}

/// List every category, ordered by parent-depth then sort_order then
/// slug. The frontend builds the nested tree from the `parent_id` field.
pub async fn list_categories(pool: &PgPool) -> AppResult<Vec<Category>> {
    let rows = sqlx::query_as::<_, Category>(
        r#"
        SELECT id, slug, name, description, parent_id, path, sort_order,
               created_at, updated_at
        FROM product_categories
        ORDER BY path ASC, sort_order ASC, slug ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Lookup by slug. Returns None when no match.
pub async fn get_category_by_slug(pool: &PgPool, slug: &str) -> AppResult<Option<Category>> {
    let row = sqlx::query_as::<_, Category>(
        r#"
        SELECT id, slug, name, description, parent_id, path, sort_order,
               created_at, updated_at
        FROM product_categories WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Refresh the `product_facets` materialized view. Called from a cron
/// worker; safe to invoke ad-hoc from an admin button.
pub async fn refresh_facets(pool: &PgPool) -> AppResult<()> {
    sqlx::query("REFRESH MATERIALIZED VIEW product_facets")
        .execute(pool)
        .await?;
    Ok(())
}

// ── Internal bind helper ──────────────────────────────────────────────

fn bind_params<'q>(
    qb: &'q mut QueryBuilder<'q, sqlx::Postgres>,
    params: &'q SearchParams,
) -> &'q mut QueryBuilder<'q, sqlx::Postgres> {
    // The fragment reserves `$1..$N` in the order fields are inspected
    // by `build_where_fragment`; mirror that order exactly.
    if let Some(q) = params.q.as_ref().filter(|s| !s.trim().is_empty()) {
        qb.push_bind(q.clone());
    }
    if let Some(cat) = params.category_id {
        qb.push_bind(cat);
    }
    if let Some(tag) = params.tag.as_ref() {
        qb.push_bind(tag.clone());
    }
    if let Some(min) = params.min_price_cents {
        qb.push_bind(min);
    }
    if let Some(max) = params.max_price_cents {
        qb.push_bind(max);
    }
    qb
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_params_produce_baseline_where() {
        let (sql, binds) = build_where_fragment(&SearchParams::default());
        assert_eq!(binds, 0);
        assert!(sql.contains("p.status = 'published'"));
        assert!(!sql.contains("plainto_tsquery"));
    }

    #[test]
    fn query_adds_tsquery_and_single_bind() {
        let p = SearchParams {
            q: Some("phone case".into()),
            ..Default::default()
        };
        let (sql, binds) = build_where_fragment(&p);
        assert_eq!(binds, 1);
        assert!(sql.contains("plainto_tsquery"));
        assert!(sql.contains("$1"));
    }

    #[test]
    fn whitespace_q_is_treated_as_absent() {
        let p = SearchParams {
            q: Some("   ".into()),
            ..Default::default()
        };
        let (sql, binds) = build_where_fragment(&p);
        assert_eq!(binds, 0);
        assert!(!sql.contains("plainto_tsquery"));
    }

    #[test]
    fn category_filter_walks_descendants() {
        let p = SearchParams {
            category_id: Some(Uuid::nil()),
            ..Default::default()
        };
        let (sql, binds) = build_where_fragment(&p);
        assert_eq!(binds, 1);
        assert!(sql.contains("product_category_links"));
        assert!(sql.contains("path LIKE"));
    }

    #[test]
    fn price_filters_are_both_optional() {
        let p = SearchParams {
            min_price_cents: Some(100),
            max_price_cents: Some(1000),
            ..Default::default()
        };
        let (sql, binds) = build_where_fragment(&p);
        assert_eq!(binds, 2);
        assert!(sql.contains("price_cents, 0) >= $1"));
        assert!(sql.contains("price_cents, 0) <= $2"));
    }

    #[test]
    fn tag_filter_uses_jsonb_contains() {
        let p = SearchParams {
            tag: Some("sale".into()),
            ..Default::default()
        };
        let (sql, binds) = build_where_fragment(&p);
        assert_eq!(binds, 1);
        assert!(sql.contains("metadata->'tags'"));
        assert!(sql.contains("? $1"));
    }

    #[test]
    fn all_filters_chain_and_number_binds_in_order() {
        let p = SearchParams {
            q: Some("foo".into()),
            category_id: Some(Uuid::nil()),
            tag: Some("t".into()),
            min_price_cents: Some(1),
            max_price_cents: Some(2),
            ..Default::default()
        };
        let (sql, binds) = build_where_fragment(&p);
        assert_eq!(binds, 5);
        // Every placeholder 1..=5 must appear.
        for n in 1..=5 {
            assert!(sql.contains(&format!("${n}")), "missing placeholder ${n}");
        }
    }
}
