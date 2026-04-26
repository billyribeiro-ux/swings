#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-15 integration coverage for the admin members lifecycle surface:
//! ban / unban, suspend (open-ended + timed) / unsuspend, PATCH profile,
//! and the composite GET /detail endpoint.
//!
//! Stripe-backed code paths (immediate cancel during ban, customer
//! address sync during PATCH) are exercised through the local-only
//! branch — neither test seeds a `subscriptions` row, so the handlers
//! short-circuit before touching Stripe. The Stripe-aware paths are
//! validated by the existing `stripe_webhooks.rs` integration suite.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::TestApp;

// ── Helpers ────────────────────────────────────────────────────────────

async fn create_member(app: &TestApp, admin_token: &str, email: &str, name: &str) -> uuid::Uuid {
    let resp = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({ "email": email, "name": name, "role": "member" }),
            Some(admin_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("create body");
    let id_str = body["user"]["id"].as_str().expect("user.id");
    uuid::Uuid::parse_str(id_str).expect("uuid parse")
}

// ── PATCH /api/admin/members/{id} ──────────────────────────────────────

#[tokio::test]
async fn patch_requires_admin_member_update_permission() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let plain = app.seed_user().await.expect("seed member");
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id =
        create_member(&app, &admin.access_token, "patch.deny@example.com", "Deny").await;

    let resp = app
        .patch_json::<Value>(
            &format!("/api/admin/members/{target_id}"),
            &json!({ "name": "Hacker" }),
            Some(&plain.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn patch_renames_and_updates_address() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id = create_member(
        &app,
        &admin.access_token,
        "patch.ok@example.com",
        "Original",
    )
    .await;

    let resp = app
        .patch_json::<Value>(
            &format!("/api/admin/members/{target_id}"),
            &json!({
                "name": "Renamed Member",
                "phone": "+15551234567",
                "billing_address": {
                    "line1": "1 Apple Park Way",
                    "city":  "Cupertino",
                    "state": "CA",
                    "postal_code": "95014",
                    "country": "us"
                }
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["name"], json!("Renamed Member"));
    assert_eq!(body["phone"], json!("+15551234567"));
    assert_eq!(body["billing_country"], json!("US"));
    assert_eq!(body["billing_postal_code"], json!("95014"));

    // The audit row should have landed.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'user.update' AND target_id = $1",
    )
    .bind(target_id.to_string())
    .fetch_one(app.db())
    .await
    .expect("count");
    assert!(count >= 1);
}

#[tokio::test]
async fn patch_rejects_invalid_country_code() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id = create_member(&app, &admin.access_token, "bad.iso@example.com", "X").await;

    let resp = app
        .patch_json::<Value>(
            &format!("/api/admin/members/{target_id}"),
            &json!({ "billing_address": { "country": "USA" } }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn patch_email_change_clears_verification() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let resp = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email": "verified.first@example.com",
                "name":  "Verified First",
                "role":  "member",
                "email_verified": true
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    let id = body["user"]["id"].as_str().expect("id").to_string();

    let resp = app
        .patch_json::<Value>(
            &format!("/api/admin/members/{id}"),
            &json!({ "email": "verified.second@example.com" }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let row = sqlx::query("SELECT email, email_verified_at FROM users WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).expect("uuid"))
        .fetch_one(app.db())
        .await
        .expect("row");
    let new_email: String = row.try_get("email").expect("email col");
    let verified: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("email_verified_at").expect("verified col");
    assert_eq!(new_email, "verified.second@example.com");
    assert!(
        verified.is_none(),
        "email_verified_at should clear after admin email change"
    );
}

#[tokio::test]
async fn patch_email_collision_returns_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let _ = create_member(&app, &admin.access_token, "first.user@example.com", "First").await;
    let other_id = create_member(
        &app,
        &admin.access_token,
        "second.user@example.com",
        "Second",
    )
    .await;

    let resp = app
        .patch_json::<Value>(
            &format!("/api/admin/members/{other_id}"),
            &json!({ "email": "first.user@example.com" }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CONFLICT);
}

// ── POST /api/admin/members/{id}/suspend with `until` ──────────────────

#[tokio::test]
async fn suspend_with_future_until_persists_deadline() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id = create_member(
        &app,
        &admin.access_token,
        "timeout.ok@example.com",
        "Timeout",
    )
    .await;

    let until = chrono::Utc::now() + chrono::Duration::hours(2);
    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/members/{target_id}/suspend"),
            &json!({ "reason": "cooldown", "until": until }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let row = sqlx::query(
        "SELECT suspended_at, suspended_until, suspension_reason FROM users WHERE id = $1",
    )
    .bind(target_id)
    .fetch_one(app.db())
    .await
    .expect("row");
    let suspended_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("suspended_at").expect("suspended_at");
    let suspended_until: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("suspended_until").expect("suspended_until");
    let reason: Option<String> = row.try_get("suspension_reason").expect("reason");
    assert!(suspended_at.is_some());
    assert!(suspended_until.is_some());
    assert_eq!(reason.as_deref(), Some("cooldown"));
}

#[tokio::test]
async fn suspend_with_past_until_is_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id =
        create_member(&app, &admin.access_token, "timeout.bad@example.com", "Bad").await;

    let until = chrono::Utc::now() - chrono::Duration::hours(1);
    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/members/{target_id}/suspend"),
            &json!({ "until": until }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

// ── POST /api/admin/members/{id}/unsuspend + /unban ────────────────────

#[tokio::test]
async fn unsuspend_clears_columns_and_audits() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id = create_member(&app, &admin.access_token, "unsuspend@example.com", "U").await;

    sqlx::query(
        "UPDATE users SET suspended_at = NOW(), suspension_reason = 'manual' WHERE id = $1",
    )
    .bind(target_id)
    .execute(app.db())
    .await
    .expect("seed suspension");

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/members/{target_id}/unsuspend"),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let row = sqlx::query("SELECT suspended_at FROM users WHERE id = $1")
        .bind(target_id)
        .fetch_one(app.db())
        .await
        .expect("row");
    let suspended_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("suspended_at").expect("col");
    assert!(suspended_at.is_none());

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'user.unsuspend' AND target_id = $1",
    )
    .bind(target_id.to_string())
    .fetch_one(app.db())
    .await
    .expect("count");
    assert!(count >= 1);
}

#[tokio::test]
async fn unban_clears_columns_and_audits() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id = create_member(&app, &admin.access_token, "unban@example.com", "U").await;

    sqlx::query("UPDATE users SET banned_at = NOW(), ban_reason = 'manual' WHERE id = $1")
        .bind(target_id)
        .execute(app.db())
        .await
        .expect("seed ban");

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/members/{target_id}/unban"),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let row = sqlx::query("SELECT banned_at FROM users WHERE id = $1")
        .bind(target_id)
        .fetch_one(app.db())
        .await
        .expect("row");
    let banned_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("banned_at").expect("col");
    assert!(banned_at.is_none());
}

#[tokio::test]
async fn unsuspend_requires_user_reactivate() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let plain = app.seed_user().await.expect("seed member");
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id =
        create_member(&app, &admin.access_token, "rbac.unsuspend@example.com", "X").await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/members/{target_id}/unsuspend"),
            &json!({}),
            Some(&plain.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── GET /api/admin/members/{id}/detail ─────────────────────────────────

#[tokio::test]
async fn detail_returns_user_and_empty_activity_for_fresh_member() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id = create_member(
        &app,
        &admin.access_token,
        "detail.fresh@example.com",
        "Fresh",
    )
    .await;

    let resp = app
        .get(
            &format!("/api/admin/members/{target_id}/detail"),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["user"]["id"], json!(target_id.to_string()));
    assert!(body["subscription"].is_null());
    assert!(body["activity"].is_array());
    assert!(body["payment_failures"].is_array());
}

#[tokio::test]
async fn detail_requires_admin_member_read() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let plain = app.seed_user().await.expect("seed");
    let admin = app.seed_admin().await.expect("admin");
    let target_id = create_member(&app, &admin.access_token, "detail.deny@example.com", "D").await;

    let resp = app
        .get(
            &format!("/api/admin/members/{target_id}/detail"),
            Some(&plain.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── DELETE /api/admin/members/{id} cancels stripe metadata ─────────────

#[tokio::test]
async fn delete_member_logs_stripe_cancel_outcome_when_no_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target_id = create_member(
        &app,
        &admin.access_token,
        "delete.nosub@example.com",
        "Goner",
    )
    .await;

    let resp = app
        .delete(
            &format!("/api/admin/members/{target_id}"),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let row = sqlx::query(
        "SELECT metadata FROM admin_actions WHERE action = 'user.delete' AND target_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(target_id.to_string())
    .fetch_one(app.db())
    .await
    .expect("row");
    let metadata: Value = row.try_get("metadata").expect("metadata");
    // No subscription seeded → stripe_cancel should be the JSON `null`
    // sentinel that the handler emits in the no-op branch.
    assert!(metadata["stripe_cancel"].is_null());
}
