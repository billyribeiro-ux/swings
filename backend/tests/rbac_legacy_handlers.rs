#![deny(warnings)]
#![forbid(unsafe_code)]

//! Phase 1.3 / Phase 3 RBAC closure: every mutating handler in the ten
//! "legacy" admin files (blog, courses, coupons, popups, products, pricing,
//! forms, admin_consent, notifications, outbox) MUST refuse a `support`-role
//! token. Today the `AdminUser` extractor enforces `role == "admin"` so the
//! gate fires at the extractor layer; tomorrow, when the handlers migrate to
//! `PrivilegedUser`, the explicit `policy.require()` calls added to each
//! handler will continue to enforce the same 403.
//!
//! Skipped automatically when neither `DATABASE_URL_TEST` nor
//! `DATABASE_URL` is set (matches the existing harness convention).
//!
//! Tests are bundled in one file (instead of one-per-handler) because each
//! test follows the identical 3-line shape — `app.seed_support()`, hit the
//! mutating route, assert `403 Forbidden`. Bundling keeps the harness boot
//! cost amortised across the matrix.

mod support;

use axum::http::StatusCode;
use serde_json::json;
use support::{TestApp, TestRole};
use uuid::Uuid;

// ── blog ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_blog_create_post_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/blog/posts",
            &json!({
                "title": "Should not be allowed",
                "content": "<p>x</p>",
                "slug": "rbac-test"
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    assert_no_blog_post(app.db(), "rbac-test").await;
}

#[tokio::test]
async fn rbac_blog_create_category_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/blog/categories",
            &json!({ "name": "RbacTest", "slug": "rbac-test" }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rbac_blog_delete_media_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .delete(
            &format!("/api/admin/blog/media/{}", Uuid::new_v4()),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── courses ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_courses_create_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/courses",
            &json!({ "title": "RBAC", "slug": "rbac" }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM courses WHERE slug = $1")
        .bind("rbac")
        .fetch_one(app.db())
        .await
        .expect("count courses");
    assert_eq!(count, 0, "support must not create a course row");
}

#[tokio::test]
async fn rbac_courses_delete_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .delete(
            &format!("/api/admin/courses/{}", Uuid::new_v4()),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── coupons ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_coupons_create_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/coupons",
            &json!({
                "code": "RBACTEST",
                "discount_type": "percentage",
                "discount_value": 10.0
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM coupons WHERE code = $1")
        .bind("RBACTEST")
        .fetch_one(app.db())
        .await
        .expect("count coupons");
    assert_eq!(count, 0, "support must not create a coupon row");
}

#[tokio::test]
async fn rbac_coupons_bulk_create_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/coupons/bulk",
            &json!({
                "count": 3,
                "discount_type": "percentage",
                "discount_value": 10.0,
                "prefix": "RBAC"
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── popups ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_popups_create_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/popups/",
            &json!({ "name": "Rbac Popup" }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM popups WHERE name = $1")
        .bind("Rbac Popup")
        .fetch_one(app.db())
        .await
        .expect("count popups");
    assert_eq!(count, 0, "support must not create a popup row");
}

#[tokio::test]
async fn rbac_popups_delete_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .delete(
            &format!("/api/admin/popups/{}", Uuid::new_v4()),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── products ────────────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_products_create_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/products",
            &json!({
                "slug": "rbac-product",
                "name": "RBAC Product",
                "product_type": "digital",
                "price_cents": 100
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE slug = $1")
        .bind("rbac-product")
        .fetch_one(app.db())
        .await
        .expect("count products");
    assert_eq!(count, 0, "support must not create a product row");
}

#[tokio::test]
async fn rbac_products_set_status_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            &format!("/api/admin/products/{}/status", Uuid::new_v4()),
            &json!({ "status": "published" }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── pricing ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_pricing_create_plan_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/pricing/plans",
            &json!({
                "name": "RBAC Plan",
                "slug": "rbac-plan",
                "amount_cents": 100,
                "currency": "usd",
                "interval": "month"
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pricing_plans WHERE slug = $1")
        .bind("rbac-plan")
        .fetch_one(app.db())
        .await
        .expect("count pricing_plans");
    assert_eq!(count, 0, "support must not create a pricing plan");
}

#[tokio::test]
async fn rbac_pricing_toggle_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            &format!("/api/admin/pricing/plans/{}/toggle", Uuid::new_v4()),
            &json!({}),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── forms ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_forms_bulk_update_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            &format!("/api/admin/forms/{}/submissions/bulk", Uuid::new_v4()),
            &json!({
                "ids": [Uuid::new_v4()],
                "action": "delete"
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── admin_consent ───────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_consent_create_banner_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/consent/banners",
            &json!({
                "region": "EU",
                "locale": "en",
                "layout": "modal",
                "position": "center",
                "copy_json": {}
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM consent_banner_configs WHERE region = $1")
            .bind("EU")
            .fetch_one(app.db())
            .await
            .expect("count consent banners");
    assert_eq!(count, 0, "support must not create a consent banner row");
}

#[tokio::test]
async fn rbac_consent_create_category_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/consent/categories",
            &json!({
                "key": "rbac_test",
                "label": "RBAC",
                "description": "should not land",
                "is_required": false,
                "sort_order": 99
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM consent_categories WHERE key = $1")
        .bind("rbac_test")
        .fetch_one(app.db())
        .await
        .expect("count consent categories");
    assert_eq!(count, 0, "support must not create a consent category");
}

// ── notifications ───────────────────────────────────────────────────────

#[tokio::test]
async fn rbac_notifications_create_template_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/notifications/templates",
            &json!({
                "key": "rbac.test",
                "channel": "email",
                "body_source": "<p>nope</p>"
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_templates WHERE key = $1")
            .bind("rbac.test")
            .fetch_one(app.db())
            .await
            .expect("count notification templates");
    assert_eq!(count, 0, "support must not create a template row");
}

#[tokio::test]
async fn rbac_notifications_add_suppression_forbidden_for_support() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            "/api/admin/notifications/suppression",
            &json!({
                "email": "rbac@example.test",
                "reason": "should not land"
            }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_suppression WHERE email = $1")
            .bind("rbac@example.test")
            .fetch_one(app.db())
            .await
            .unwrap_or(0);
    assert_eq!(count, 0, "support must not add a suppression entry");
}

// ── outbox ──────────────────────────────────────────────────────────────
//
// Outbox is a special case: the FDN-07 seed grants `support`
// `admin.outbox.read` AND `admin.outbox.retry`. The handlers' own gate is
// the strict `AdminUser` extractor (role == "admin"), so support is still
// rejected at the door — proving the rest of the per-action perms hold the
// line if a future patch relaxes the extractor to `PrivilegedUser`.
//
// Tests assert the AdminUser extractor's 403 today; if/when the handlers
// migrate to `PrivilegedUser`, swap these to use a `member`-role token to
// keep the gate exercised (members lack both `admin.dashboard.read` and
// `admin.outbox.*`).

#[tokio::test]
async fn rbac_outbox_list_forbidden_for_support_via_admin_user_gate() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .get("/api/admin/outbox/", Some(&support.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rbac_outbox_retry_forbidden_for_support_via_admin_user_gate() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .post_json(
            &format!("/api/admin/outbox/{}/retry", Uuid::new_v4()),
            &json!({}),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── policy.require defense in depth ─────────────────────────────────────
//
// Demonstrate that the policy.require gate added to every legacy mutator
// fires *independently* of the extractor: forge an "admin"-role JWT for a
// user whose policy snapshot lacks the perm, and assert the same 403. We
// achieve this by reloading the test app's PolicyHandle with a snapshot
// that grants the admin role nothing — every protected mutator still runs
// through the extractor (because `role == "admin"` literally) but trips
// `policy.require` and returns Forbidden. Catches regressions where a
// future refactor accidentally drops the `admin.require()` line.

#[tokio::test]
async fn rbac_blog_create_post_forbidden_for_admin_when_perm_revoked() {
    use swings_api::authz::Policy;
    use swings_api::models::UserRole;

    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app
        .seed_user_with_role(TestRole::Admin)
        .await
        .expect("seed admin");

    // Wipe `admin.<resource>.<verb>` from the in-memory policy snapshot and
    // keep only `admin.dashboard.read` so the handler is reached but the
    // explicit `admin.require("blog.post.create")` call short-circuits.
    let stripped = Policy::from_pairs([(UserRole::Admin, "admin.dashboard.read")]);
    app.replace_policy_for_tests(stripped);

    let resp = app
        .post_json(
            "/api/admin/blog/posts",
            &json!({
                "title": "Defense in depth",
                "content": "<p>x</p>",
                "slug": "rbac-perm-revoked"
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
    assert_no_blog_post(app.db(), "rbac-perm-revoked").await;
}

// ── helpers ─────────────────────────────────────────────────────────────

async fn assert_no_blog_post(pool: &sqlx::PgPool, slug: &str) {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blog_posts WHERE slug = $1")
        .bind(slug)
        .fetch_one(pool)
        .await
        .expect("count blog_posts");
    assert_eq!(count, 0, "support must not create a blog_post row");
}
