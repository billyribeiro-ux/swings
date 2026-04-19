#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-14 integration coverage for the admin audit-log viewer.
//!
//! Strategy: seed a handful of `admin_actions` rows directly via SQL
//! (the writer is `services::audit::record_admin_action`, but we want
//! deterministic actor/action/metadata combinations the FTS indexes
//! can match against). Then exercise:
//!
//!   * RBAC matrix (member / support / admin / export-only).
//!   * Free-text search via `q=`.
//!   * Exact action / actor / target_kind filters.
//!   * `target_id` substring + `metadata_contains` JSON-path probe.
//!   * Date-range bounds + invalid-bound 400.
//!   * Pagination metadata (`page`, `per_page`, `total_pages`).
//!   * 404 on unknown id; 200 on read happy path.
//!   * CSV export header + row counts + audit-of-audit log entry.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use support::TestApp;
use uuid::Uuid;

async fn seed_action(
    pool: &PgPool,
    actor_id: Uuid,
    actor_role: &str,
    action: &str,
    target_kind: &str,
    target_id: Option<&str>,
    metadata: Value,
) -> Uuid {
    let id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO admin_actions
            (actor_id, actor_role, action, target_kind, target_id, metadata)
        VALUES
            ($1, $2::user_role, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(actor_id)
    .bind(actor_role)
    .bind(action)
    .bind(target_kind)
    .bind(target_id)
    .bind(metadata)
    .fetch_one(pool)
    .await
    .expect("insert admin_action");
    id.0
}

// ── RBAC ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_read_audit_log() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed");

    let resp = app
        .get("/api/admin/audit", Some(&member.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn support_can_read_but_cannot_export() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed");

    let list = app
        .get("/api/admin/audit", Some(&support.access_token))
        .await;
    list.assert_status(StatusCode::OK);

    let csv = app
        .get(
            "/api/admin/audit/export.csv",
            Some(&support.access_token),
        )
        .await;
    csv.assert_status(StatusCode::FORBIDDEN);
}

// ── List / search ──────────────────────────────────────────────────────

#[tokio::test]
async fn free_text_search_matches_action_and_metadata() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    seed_action(
        app.db(),
        admin.id,
        "admin",
        "user.suspend",
        "user",
        Some("uuid-aaa"),
        json!({"reason": "fraud_review"}),
    )
    .await;
    seed_action(
        app.db(),
        admin.id,
        "admin",
        "subscription.cancel",
        "subscription",
        Some("uuid-bbb"),
        json!({"reason": "billing"}),
    )
    .await;

    // q hits the action token.
    let by_action = app
        .get(
            "/api/admin/audit?q=suspend",
            Some(&admin.access_token),
        )
        .await;
    by_action.assert_status(StatusCode::OK);
    let body: Value = by_action.json().expect("body");
    assert!(body["total"].as_i64().unwrap_or(0) >= 1);
    let rows = body["data"].as_array().expect("rows");
    assert!(rows.iter().any(|r| r["action"] == "user.suspend"));

    // q hits a metadata token.
    let by_meta = app
        .get(
            "/api/admin/audit?q=fraud_review",
            Some(&admin.access_token),
        )
        .await;
    by_meta.assert_status(StatusCode::OK);
    let body: Value = by_meta.json().expect("body");
    assert!(body["total"].as_i64().unwrap_or(0) >= 1);
}

#[tokio::test]
async fn exact_filters_compose() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let other = app.seed_admin().await.expect("other");

    seed_action(
        app.db(),
        admin.id,
        "admin",
        "user.suspend",
        "user",
        Some("aaa-target"),
        json!({"reason": "x"}),
    )
    .await;
    seed_action(
        app.db(),
        other.id,
        "admin",
        "user.suspend",
        "user",
        Some("bbb-target"),
        json!({"reason": "x"}),
    )
    .await;

    let url = format!(
        "/api/admin/audit?actor_id={}&action=user.suspend&target_kind=user",
        admin.id
    );
    let resp = app.get(&url, Some(&admin.access_token)).await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let rows = body["data"].as_array().expect("rows");
    assert!(rows
        .iter()
        .all(|r| r["actor_id"].as_str() == Some(&admin.id.to_string())));
    assert!(rows
        .iter()
        .all(|r| r["action"] == "user.suspend"));
}

#[tokio::test]
async fn target_id_substring_uses_trigram_index() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    seed_action(
        app.db(),
        admin.id,
        "admin",
        "order.refund",
        "order",
        Some("ord_2026_apr_xyz_001"),
        json!({"amount_cents": 1500}),
    )
    .await;
    seed_action(
        app.db(),
        admin.id,
        "admin",
        "order.refund",
        "order",
        Some("ord_2026_mar_abc_002"),
        json!({"amount_cents": 2500}),
    )
    .await;

    let resp = app
        .get(
            "/api/admin/audit?target_id=apr_xyz",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let rows = body["data"].as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]["target_id"].as_str(),
        Some("ord_2026_apr_xyz_001")
    );
}

#[tokio::test]
async fn metadata_contains_filters_via_jsonb_containment() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    seed_action(
        app.db(),
        admin.id,
        "admin",
        "user.update",
        "user",
        Some("u-1"),
        json!({"reason": "fraud", "channel": "email"}),
    )
    .await;
    seed_action(
        app.db(),
        admin.id,
        "admin",
        "user.update",
        "user",
        Some("u-2"),
        json!({"reason": "billing", "channel": "email"}),
    )
    .await;

    let mc = "%7B%22reason%22%3A%22fraud%22%7D"; // {"reason":"fraud"}
    let resp = app
        .get(
            &format!("/api/admin/audit?metadata_contains={mc}"),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let rows = body["data"].as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["target_id"].as_str(), Some("u-1"));
}

#[tokio::test]
async fn metadata_contains_invalid_json_is_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    let resp = app
        .get(
            "/api/admin/audit?metadata_contains=not-json",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn metadata_contains_non_object_is_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    // Encoded `[1, 2, 3]` — valid JSON but not an object.
    let resp = app
        .get(
            "/api/admin/audit?metadata_contains=%5B1%2C2%2C3%5D",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn from_after_to_is_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    let resp = app
        .get(
            "/api/admin/audit?from=2026-04-19T00:00:00Z&to=2026-04-18T00:00:00Z",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn pagination_metadata_is_consistent() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    for i in 0..5 {
        seed_action(
            app.db(),
            admin.id,
            "admin",
            "page.test",
            "page",
            Some(&format!("p-{i}")),
            json!({"i": i}),
        )
        .await;
    }

    let resp = app
        .get(
            "/api/admin/audit?action=page.test&limit=2&offset=2",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["total"], json!(5));
    assert_eq!(body["per_page"], json!(2));
    assert_eq!(body["page"], json!(2));
    assert_eq!(body["total_pages"], json!(3));
    assert_eq!(body["data"].as_array().expect("rows").len(), 2);
}

#[tokio::test]
async fn read_one_returns_row_or_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    let id = seed_action(
        app.db(),
        admin.id,
        "admin",
        "single.read",
        "user",
        Some("solo"),
        json!({"k": "v"}),
    )
    .await;

    let ok = app
        .get(
            &format!("/api/admin/audit/{id}"),
            Some(&admin.access_token),
        )
        .await;
    ok.assert_status(StatusCode::OK);
    let body: Value = ok.json().expect("body");
    assert_eq!(body["action"], json!("single.read"));
    assert_eq!(body["target_id"], json!("solo"));

    let nope = app
        .get(
            &format!("/api/admin/audit/{}", Uuid::new_v4()),
            Some(&admin.access_token),
        )
        .await;
    nope.assert_status(StatusCode::NOT_FOUND);
}

// ── CSV export ─────────────────────────────────────────────────────────

#[tokio::test]
async fn csv_export_returns_csv_and_logs_audit_of_audit() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    for _ in 0..3 {
        seed_action(
            app.db(),
            admin.id,
            "admin",
            "csv.test",
            "thing",
            Some("t"),
            json!({"a": 1}),
        )
        .await;
    }

    let resp = app
        .get(
            "/api/admin/audit/export.csv?action=csv.test",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    assert_eq!(
        resp.header("content-type").unwrap_or_default(),
        "text/csv; charset=utf-8"
    );
    let body = resp.text();
    let mut lines = body.lines();
    let header = lines.next().expect("header");
    assert!(header.starts_with("id,created_at,actor_id,actor_role,action"));
    let data_rows: Vec<&str> = lines.filter(|l| !l.is_empty()).collect();
    assert_eq!(data_rows.len(), 3, "expected 3 csv data rows");

    // Audit-of-audit logged with row count and filter snapshot.
    let logged: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions
          WHERE action = 'admin.audit.export'
            AND (metadata->>'rows')::int = 3",
    )
    .fetch_one(app.db())
    .await
    .expect("audit row");
    assert_eq!(logged, 1);
}
