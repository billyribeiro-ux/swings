//! Regression coverage for `POST /api/admin/blog/posts/{id}/autosave`.
//!
//! Background — Hard Rule 4 mandates that every admin mutation emit an
//! audit row in `admin_actions`. `admin_autosave_post` mutates the
//! `blog_posts` row (overwrites `title`, `content`, `content_json`,
//! `word_count`, `reading_time_minutes`, `updated_at`) but historically
//! emitted no audit. This test seeds a post, hits the autosave endpoint,
//! and asserts an `admin_actions` row appears with `action =
//! 'blog.post.autosave'`. Splitting from the explicit-update action key
//! is intentional so audit retention can downsample the high-volume
//! autosave stream without losing the explicit edits.

mod support;

use reqwest::StatusCode;
use serde_json::{json, Value};
use support::TestApp;
use uuid::Uuid;

#[tokio::test]
async fn autosave_writes_admin_actions_row_with_distinct_action_key() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // 1. Seed a post via the create endpoint so we have a real id.
    let create_body = json!({
        "title": "Autosave audit test — initial",
        "content": "<p>initial body</p>",
        "slug": "regression-autosave-audit",
    });
    let resp = app
        .post_json_with_idempotency_key(
            "/api/admin/blog/posts",
            &create_body,
            Some(&admin.access_token),
            "regr-autosave-create-1",
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let created: Value = resp.json().expect("json body");
    let post_id = created["id"].as_str().expect("post id").to_string();
    let post_uuid: Uuid = post_id.parse().expect("post id is uuid");

    // 2. Capture pre-state: how many autosave audit rows currently exist
    // for THIS post id. Post-create may have written a `blog.post.create`
    // row; we don't care — we filter by `action = 'blog.post.autosave'`
    // and target_id so the assertion is precise.
    let pre_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions \
         WHERE action = 'blog.post.autosave' AND target_id = $1",
    )
    .bind(post_uuid)
    .fetch_one(app.db())
    .await
    .expect("count pre-state");
    assert_eq!(pre_count, 0, "no autosave rows should exist yet");

    // 3. Hit the autosave endpoint.
    let autosave_body = json!({
        "title": "Autosave audit test — typing in progress",
        "content": "<p>partially typed body</p>",
    });
    let resp = app
        .post_json(
            &format!("/api/admin/blog/posts/{post_id}/autosave"),
            &autosave_body,
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4. Verify a `blog.post.autosave` audit row landed for THIS post.
    let row: (Uuid, String, Option<Uuid>, Uuid, serde_json::Value) = sqlx::query_as(
        "SELECT id, action, target_id, actor_user_id, metadata \
         FROM admin_actions \
         WHERE action = 'blog.post.autosave' AND target_id = $1",
    )
    .bind(post_uuid)
    .fetch_one(app.db())
    .await
    .expect("autosave audit row must exist after POST /autosave");

    let (_audit_id, action, target_id, actor_user_id, metadata) = row;
    assert_eq!(action, "blog.post.autosave");
    assert_eq!(target_id, Some(post_uuid));
    assert_eq!(actor_user_id, admin.id);
    // Metadata must include the post slug + ownership flag so investigators
    // can correlate without re-joining to blog_posts.
    assert_eq!(
        metadata.get("slug").and_then(|v| v.as_str()),
        Some("regression-autosave-audit")
    );
    assert!(
        metadata.get("owned_by_actor").is_some(),
        "metadata must carry owned_by_actor flag"
    );

    // 5. Crucially, the action key must be DISTINCT from explicit updates.
    // If a future refactor accidentally folds autosave into
    // `blog.post.update`, audit retention's downsampling rule (which
    // targets `blog.post.autosave` specifically) silently breaks.
    let update_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions \
         WHERE action = 'blog.post.update' AND target_id = $1",
    )
    .bind(post_uuid)
    .fetch_one(app.db())
    .await
    .expect("count update rows");
    assert_eq!(
        update_count, 0,
        "autosave must not emit a `blog.post.update` row \
         (use `blog.post.autosave` so audit retention can downsample)"
    );
}
