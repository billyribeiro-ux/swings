//! Regression coverage for `PUT /api/admin/blog/posts/{id}`.
//!
//! Background — `db::create_blog_revision` was decoding the bumped
//! revision number as `(i64,)` while `blog_revisions.revision_number` is
//! `INT4` in the schema (`002_blog.sql`). `COALESCE(MAX(int4), 0) + 1`
//! returns INT4, so the read failed with
//! `ColumnDecode { i64 vs INT4 }` and the entire `admin_update_post`
//! handler 500'd whenever it tried to snapshot the previous version.
//!
//! The test seeds a post via the create endpoint, then PUTs an update
//! TWICE — the second call exercises the revision INSERT (the first
//! create has nothing to revise). Both must succeed with HTTP 200.

mod support;

use reqwest::StatusCode;
use serde_json::{json, Value};
use support::TestApp;

#[tokio::test]
async fn update_blog_post_twice_succeeds_and_writes_revisions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // 1. Create a post.
    let create_body = json!({
        "title": "Initial title",
        "content": "<p>v1 body</p>",
        "slug": "regression-blog-update",
    });
    let resp = app
        .post_json_with_idempotency_key(
            "/api/admin/blog/posts",
            &create_body,
            Some(&admin.access_token),
            "regr-blog-create-1",
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let created: Value = resp.json().expect("json body");
    let post_id = created["id"].as_str().expect("post id").to_string();

    // 2. First PUT — exercises `create_blog_revision` for the first time
    // (existing post had zero revisions; COALESCE branch returns 0, +1 → 1).
    let upd1 = json!({
        "title": "Updated v2",
        "content": "<p>v2 body</p>",
    });
    let resp1 = app
        .put_json(
            &format!("/api/admin/blog/posts/{post_id}"),
            &upd1,
            Some(&admin.access_token),
        )
        .await;
    resp1.assert_status(StatusCode::OK);

    // 3. Second PUT — now there IS a row in blog_revisions, so
    // `MAX(revision_number)` returns INT4 from the table. Pre-fix this
    // 500'd with `ColumnDecode { i64 vs INT4 }`. Post-fix it's clean.
    let upd2 = json!({
        "title": "Updated v3",
        "content": "<p>v3 body</p>",
    });
    let resp2 = app
        .put_json(
            &format!("/api/admin/blog/posts/{post_id}"),
            &upd2,
            Some(&admin.access_token),
        )
        .await;
    resp2.assert_status(StatusCode::OK);

    // 4. Verify two revisions were written, with consecutive numbers.
    let rows: Vec<(i32,)> = sqlx::query_as(
        "SELECT revision_number FROM blog_revisions WHERE post_id = $1::uuid ORDER BY revision_number",
    )
    .bind(&post_id)
    .fetch_all(app.db())
    .await
    .expect("query revisions");
    assert_eq!(
        rows.iter().map(|r| r.0).collect::<Vec<_>>(),
        vec![1, 2],
        "two revision rows numbered 1 + 2 must exist after two updates"
    );
}
