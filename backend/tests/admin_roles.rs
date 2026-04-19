#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-09 integration coverage for the role / permission matrix and
//! the policy hot-reload path.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::{AssertProblem, TestApp};

// ── RBAC gates ─────────────────────────────────────────────────────────

#[tokio::test]
async fn list_matrix_requires_admin_role_read() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support_user = app.seed_support().await.expect("seed support");
    // Support gets `admin.dashboard.read` so PrivilegedUser passes,
    // but the seed in 064 does not grant `admin.role.read`.
    let resp = app
        .get("/api/admin/security/roles", Some(&support_user.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_cannot_touch_role_matrix() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    let resp = app
        .get("/api/admin/security/roles", Some(&member.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── List shape ─────────────────────────────────────────────────────────

#[tokio::test]
async fn matrix_lists_all_known_roles_and_seeded_pairs() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .get("/api/admin/security/roles", Some(&admin.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("matrix body");
    let roles: Vec<&str> = body["roles"]
        .as_array()
        .expect("roles array")
        .iter()
        .map(|v| v.as_str().expect("role string"))
        .collect();
    for r in ["member", "author", "support", "admin"] {
        assert!(roles.contains(&r), "missing role {r}");
    }

    // Sanity: at least the admin self-lock guards must appear.
    let pairs: Vec<(&str, &str)> = body["matrix"]
        .as_array()
        .expect("matrix array")
        .iter()
        .map(|v| {
            (
                v["role"].as_str().expect("role"),
                v["permission"].as_str().expect("permission"),
            )
        })
        .collect();
    assert!(pairs.contains(&("admin", "admin.role.manage")));
    assert!(pairs.contains(&("admin", "admin.dashboard.read")));
}

// ── Grant / revoke + policy hot reload ─────────────────────────────────

#[tokio::test]
async fn grant_then_revoke_roundtrip_and_policy_reloads() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Grant `admin.settings.read` to support so a support seat can
    // hit the settings list.
    let grant = app
        .post_json::<Value>(
            "/api/admin/security/roles/support/admin.settings.read",
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    grant.assert_status(StatusCode::OK);

    // Verify the row landed.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_permissions WHERE role = 'support'::user_role AND permission = 'admin.settings.read'",
    )
    .fetch_one(app.db())
    .await
    .expect("count row");
    assert_eq!(count, 1);

    // Support can now list settings — proves the in-process policy
    // cache picked up the new grant without a restart.
    let support_user = app.seed_support().await.expect("seed support");
    let list = app
        .get("/api/admin/settings", Some(&support_user.access_token))
        .await;
    list.assert_status(StatusCode::OK);

    // Revoke and verify support is locked back out.
    let revoke = app
        .delete(
            "/api/admin/security/roles/support/admin.settings.read",
            Some(&admin.access_token),
        )
        .await;
    revoke.assert_status(StatusCode::OK);

    let list2 = app
        .get("/api/admin/settings", Some(&support_user.access_token))
        .await;
    list2.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn grant_unknown_permission_returns_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/security/roles/support/no.such.permission",
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_problem(AssertProblem {
        status: StatusCode::BAD_REQUEST,
        type_suffix: "bad-request",
        title: "Bad Request",
    });
}

#[tokio::test]
async fn grant_to_unknown_role_returns_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/security/roles/superadmin/admin.settings.read",
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

// ── Self-lock guards ───────────────────────────────────────────────────

#[tokio::test]
async fn cannot_revoke_admin_role_manage_from_admin() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .delete(
            "/api/admin/security/roles/admin/admin.role.manage",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CONFLICT);

    // The row must still be present.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM role_permissions WHERE role = 'admin'::user_role AND permission = 'admin.role.manage'",
    )
    .fetch_one(app.db())
    .await
    .expect("count row");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn cannot_revoke_admin_dashboard_read_from_admin() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .delete(
            "/api/admin/security/roles/admin/admin.dashboard.read",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn replace_admin_set_must_keep_self_lock_guards() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .put_json(
            "/api/admin/security/roles/admin",
            &json!({ "permissions": ["admin.dashboard.read"] }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CONFLICT);
}

// ── Atomic bulk replace ────────────────────────────────────────────────

#[tokio::test]
async fn replace_role_atomically_and_audits_once() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .put_json(
            "/api/admin/security/roles/support",
            &json!({
                "permissions": [
                    "admin.dashboard.read",
                    "admin.security.read",
                    "user.session.read",
                ]
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let rows: Vec<String> = sqlx::query(
        "SELECT permission FROM role_permissions WHERE role = 'support'::user_role ORDER BY permission",
    )
    .fetch_all(app.db())
    .await
    .expect("rows")
    .into_iter()
    .map(|r| r.try_get::<String, _>("permission").expect("perm string"))
    .collect();
    assert_eq!(
        rows,
        vec![
            "admin.dashboard.read".to_string(),
            "admin.security.read".to_string(),
            "user.session.read".to_string(),
        ]
    );

    // Exactly one audit row attributed to the bulk action.
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.role.replace' AND target_kind = 'role_permissions'",
    )
    .fetch_one(app.db())
    .await
    .expect("audit count");
    assert_eq!(audit_count, 1);
}
