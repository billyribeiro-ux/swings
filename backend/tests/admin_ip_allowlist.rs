#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-06 integration coverage for the admin IP allowlist CRUD surface
//! and the gating middleware that consults the same table.
//!
//! Every test drives a real request through the in-process Axum router so
//! the FDN-07 permission seed, the audit-log writer, and the IP-allowlist
//! middleware are exercised end-to-end against the per-test ephemeral
//! schema provisioned by `TestApp`.
//!
//! Skipped automatically when neither `DATABASE_URL_TEST` nor
//! `DATABASE_URL` is set — the existing harness convention.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::{AssertProblem, TestApp};
use uuid::Uuid;

// ── Open-mode default: empty list = pass-through ───────────────────────

#[tokio::test]
async fn empty_allowlist_allows_admin_traffic() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    // No allowlist rows → every admin endpoint must remain reachable.
    let resp = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let body: Value = resp.json().expect("list body");
    assert_eq!(body["total"].as_i64(), Some(0));
    assert!(
        body["data"].as_array().map(Vec::is_empty).unwrap_or(false),
        "data array starts empty"
    );
}

// ── CRUD happy path ─────────────────────────────────────────────────────

#[tokio::test]
async fn admin_can_create_list_toggle_and_delete_entries() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    // The harness stamps `X-Forwarded-For` with a /32 in 10.x.y.z; the only
    // IP the middleware will accept while a non-empty allowlist is active
    // is one that contains it. We therefore add 10.0.0.0/8 first so the
    // subsequent calls can survive enforcement.
    let create = app
        .post_json(
            "/api/admin/security/ip-allowlist",
            &json!({
                "cidr": "10.0.0.0/8",
                "label": "harness-traffic",
                "is_active": true,
            }),
            Some(&admin.access_token),
        )
        .await;
    create.assert_status(StatusCode::OK);
    let entry: Value = create.json().expect("create body");
    let entry_id: Uuid = entry["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("entry id is a uuid");
    assert_eq!(entry["cidr"], "10.0.0.0/8");
    assert_eq!(entry["label"], "harness-traffic");
    assert_eq!(entry["is_active"], true);

    // List reflects the new row.
    let list = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&admin.access_token),
        )
        .await;
    list.assert_status(StatusCode::OK);
    let body: Value = list.json().expect("list body");
    assert_eq!(body["total"].as_i64(), Some(1));
    assert_eq!(body["data"][0]["id"], entry["id"]);

    // Toggle deactivates the row.
    let toggled = app
        .post_json(
            &format!("/api/admin/security/ip-allowlist/{entry_id}/toggle"),
            &json!({ "is_active": false }),
            Some(&admin.access_token),
        )
        .await;
    toggled.assert_status(StatusCode::OK);
    let toggled_body: Value = toggled.json().expect("toggle body");
    assert_eq!(toggled_body["is_active"], false);

    // Delete removes the row.
    let removed = app
        .delete(
            &format!("/api/admin/security/ip-allowlist/{entry_id}"),
            Some(&admin.access_token),
        )
        .await;
    removed.assert_status(StatusCode::OK);

    let list_after = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&admin.access_token),
        )
        .await;
    list_after.assert_status(StatusCode::OK);
    let body: Value = list_after.json().expect("list body");
    assert_eq!(body["total"].as_i64(), Some(0));
}

// ── Audit trail ────────────────────────────────────────────────────────

#[tokio::test]
async fn create_entry_writes_audit_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json(
            "/api/admin/security/ip-allowlist",
            &json!({
                "cidr": "10.0.0.0/8",
                "label": "office",
                "is_active": true,
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("create body");
    let entry_id = body["id"].as_str().expect("entry id");

    let row = sqlx::query(
        "SELECT action, target_kind, target_id, metadata
           FROM admin_actions
          WHERE actor_id = $1
          ORDER BY created_at DESC
          LIMIT 1",
    )
    .bind(admin.id)
    .fetch_one(app.db())
    .await
    .expect("audit row");

    let action: String = row.get("action");
    let target_kind: String = row.get("target_kind");
    let target_id: String = row.get("target_id");
    let metadata: Value = row.get("metadata");

    assert_eq!(action, "admin.ip_allowlist.create");
    assert_eq!(target_kind, "admin_ip_allowlist");
    assert_eq!(target_id, entry_id);
    assert_eq!(metadata["cidr"], "10.0.0.0/8");
    assert_eq!(metadata["label"], "office");
    assert_eq!(metadata["is_active"], true);
}

// ── Validation ─────────────────────────────────────────────────────────

#[tokio::test]
async fn invalid_cidr_returns_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json(
            "/api/admin/security/ip-allowlist",
            &json!({
                "cidr": "not-a-cidr",
                "label": "broken",
                "is_active": true,
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_cidr_returns_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    let body = json!({
        "cidr": "10.0.0.0/8",
        "label": "first",
        "is_active": true,
    });

    let first = app
        .post_json(
            "/api/admin/security/ip-allowlist",
            &body,
            Some(&admin.access_token),
        )
        .await;
    first.assert_status(StatusCode::OK);

    let dup = app
        .post_json(
            "/api/admin/security/ip-allowlist",
            &body,
            Some(&admin.access_token),
        )
        .await;
    dup.assert_status(StatusCode::CONFLICT);
}

// ── RBAC + auth ────────────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_read_or_mutate_allowlist() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let member = app.seed_user().await.expect("seed member");

    let read = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&member.access_token),
        )
        .await;
    read.assert_problem(AssertProblem {
        status: StatusCode::FORBIDDEN,
        type_suffix: "forbidden",
        title: "Forbidden",
    });

    let create = app
        .post_json(
            "/api/admin/security/ip-allowlist",
            &json!({"cidr": "10.0.0.0/8", "label": "x"}),
            Some(&member.access_token),
        )
        .await;
    create.assert_problem(AssertProblem {
        status: StatusCode::FORBIDDEN,
        type_suffix: "forbidden",
        title: "Forbidden",
    });
}

#[tokio::test]
async fn support_cannot_read_or_mutate_allowlist() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    // Support has admin.dashboard.read → reaches PrivilegedUser, but the
    // ADM-06 permission seed restricts ip_allowlist.* to admin only. The
    // explicit `require()` call inside each handler must therefore reject.
    let support = app.seed_support().await.expect("seed support");

    let read = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&support.access_token),
        )
        .await;
    read.assert_status(StatusCode::FORBIDDEN);

    let create = app
        .post_json(
            "/api/admin/security/ip-allowlist",
            &json!({"cidr": "10.0.0.0/8", "label": "x"}),
            Some(&support.access_token),
        )
        .await;
    create.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unauthenticated_request_is_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let resp = app.get("/api/admin/security/ip-allowlist", None).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ── Middleware enforcement ─────────────────────────────────────────────

#[tokio::test]
async fn middleware_blocks_traffic_when_ip_outside_allowlist() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    // Insert an active entry that does NOT cover the harness IP (which is
    // a /32 in 10.0.0.0/8). The next admin request must be 403.
    sqlx::query(
        "INSERT INTO admin_ip_allowlist (cidr, label, is_active, created_by)
         VALUES ($1::cidr, $2, TRUE, $3)",
    )
    .bind("203.0.113.0/24")
    .bind("non-matching")
    .bind(admin.id)
    .execute(app.db())
    .await
    .expect("seed mismatching allowlist row");

    let resp = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn middleware_allows_traffic_when_ip_inside_allowlist() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    // 10.0.0.0/8 covers the harness IP — the request must succeed.
    sqlx::query(
        "INSERT INTO admin_ip_allowlist (cidr, label, is_active, created_by)
         VALUES ($1::cidr, $2, TRUE, $3)",
    )
    .bind("10.0.0.0/8")
    .bind("harness-friendly")
    .bind(admin.id)
    .execute(app.db())
    .await
    .expect("seed matching allowlist row");

    let resp = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn middleware_ignores_inactive_entries() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    // Inactive non-matching row → middleware must treat as empty list and
    // pass the request through.
    sqlx::query(
        "INSERT INTO admin_ip_allowlist (cidr, label, is_active, created_by)
         VALUES ($1::cidr, $2, FALSE, $3)",
    )
    .bind("203.0.113.0/24")
    .bind("inactive-deny")
    .bind(admin.id)
    .execute(app.db())
    .await
    .expect("seed inactive allowlist row");

    let resp = app
        .get(
            "/api/admin/security/ip-allowlist",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}
