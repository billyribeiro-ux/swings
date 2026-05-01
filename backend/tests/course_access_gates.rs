#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration tests for **course access gates**.
//!
//! Three classes of access:
//!
//!   1. **Free course (`is_free = TRUE`)**: anyone authenticated can enroll;
//!      anyone (incl. anonymous) gets full lesson content.
//!   2. **Subscription-included course (`is_included_in_subscription = TRUE`)**:
//!      enrollment requires Active or Trialing sub; lesson content is
//!      redacted for non-paying viewers (everything except `is_preview`
//!      lessons).
//!   3. **Pay-per-course (`price_cents > 0`, `is_free=FALSE`,
//!      `is_included_in_subscription=FALSE`)**: enrollment 403 until the
//!      pay-per-course flow lands (currently a hard block — better than the
//!      pre-fix behaviour of "anyone can enroll without paying").
//!
//! Plus regressions for the lesson redaction logic — `is_preview = TRUE`
//! lessons stay visible regardless, marketing depends on it.

mod support;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::Value;
use support::{TestApp, TestUser};
use swings_api::{
    db,
    models::{SubscriptionPlan, SubscriptionStatus},
};
use uuid::Uuid;

// ── Fixtures ──────────────────────────────────────────────────────────────

struct CourseFixture {
    course_id: Uuid,
    course_slug: String,
    /// Lesson visible to anyone (`is_preview = TRUE`).
    preview_lesson_id: Uuid,
    /// Lesson body must be redacted for non-paying viewers.
    locked_lesson_id: Uuid,
}

async fn seed_pricing_plan(app: &TestApp) -> Uuid {
    let id = Uuid::new_v4();
    let slug = format!("plan-{}", &id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO pricing_plans
            (id, name, slug, amount_cents, currency, interval, interval_count,
             trial_days, features, is_active)
        VALUES ($1, $2, $3, 2999, 'usd', 'month', 1, 0, '[]'::jsonb, TRUE)
        "#,
    )
    .bind(id)
    .bind(format!("Plan {}", &id.to_string()[..8]))
    .bind(slug)
    .execute(app.db())
    .await
    .expect("seed pricing_plan");
    id
}

async fn give_active_subscription(app: &TestApp, user: &TestUser) {
    let plan_id = seed_pricing_plan(app).await;
    let now = Utc::now();
    db::upsert_subscription(
        app.db(),
        user.id,
        &format!("cus_{}", Uuid::new_v4().simple()),
        &format!("sub_{}", Uuid::new_v4().simple()),
        &SubscriptionPlan::Monthly,
        &SubscriptionStatus::Active,
        now,
        now + Duration::days(30),
        Some(plan_id),
    )
    .await
    .expect("seed sub");
}

async fn give_subscription_with_status(app: &TestApp, user: &TestUser, status: SubscriptionStatus) {
    let plan_id = seed_pricing_plan(app).await;
    let now = Utc::now();
    db::upsert_subscription(
        app.db(),
        user.id,
        &format!("cus_{}", Uuid::new_v4().simple()),
        &format!("sub_{}", Uuid::new_v4().simple()),
        &SubscriptionPlan::Monthly,
        &status,
        now,
        now + Duration::days(30),
        Some(plan_id),
    )
    .await
    .expect("seed sub");
}

async fn seed_course(
    app: &TestApp,
    instructor_id: Uuid,
    is_free: bool,
    is_included_in_subscription: bool,
    price_cents: i64,
) -> CourseFixture {
    let course_id = Uuid::new_v4();
    let module_id = Uuid::new_v4();
    let preview_lesson_id = Uuid::new_v4();
    let locked_lesson_id = Uuid::new_v4();
    let suffix = &course_id.to_string()[..8];
    let course_slug = format!("course-{}", suffix);

    sqlx::query(
        r#"
        INSERT INTO courses
            (id, title, slug, description, instructor_id, price_cents, currency,
             is_free, is_included_in_subscription, published, published_at)
        VALUES ($1, $2, $3, $4, $5, $6, 'usd', $7, $8, TRUE, NOW())
        "#,
    )
    .bind(course_id)
    .bind(format!("Course {}", suffix))
    .bind(&course_slug)
    .bind("Test course description")
    .bind(instructor_id)
    .bind(price_cents)
    .bind(is_free)
    .bind(is_included_in_subscription)
    .execute(app.db())
    .await
    .expect("seed course");

    sqlx::query(
        "INSERT INTO course_modules (id, course_id, title, sort_order) VALUES ($1, $2, $3, 0)",
    )
    .bind(module_id)
    .bind(course_id)
    .bind("Module 1")
    .execute(app.db())
    .await
    .expect("seed module");

    // Preview lesson — body always visible.
    sqlx::query(
        r#"
        INSERT INTO course_lessons
            (id, module_id, title, slug, content, video_url, sort_order, is_preview)
        VALUES ($1, $2, $3, $4, $5, $6, 0, TRUE)
        "#,
    )
    .bind(preview_lesson_id)
    .bind(module_id)
    .bind("Preview lesson")
    .bind("preview")
    .bind("PREVIEW LESSON BODY")
    .bind("https://cdn.example/preview.mp4")
    .execute(app.db())
    .await
    .expect("seed preview lesson");

    // Locked lesson — body should be redacted for unauthorized viewers.
    sqlx::query(
        r#"
        INSERT INTO course_lessons
            (id, module_id, title, slug, content, video_url, sort_order, is_preview)
        VALUES ($1, $2, $3, $4, $5, $6, 1, FALSE)
        "#,
    )
    .bind(locked_lesson_id)
    .bind(module_id)
    .bind("Locked lesson")
    .bind("locked")
    .bind("PAID LESSON BODY")
    .bind("https://cdn.example/locked.mp4")
    .execute(app.db())
    .await
    .expect("seed locked lesson");

    CourseFixture {
        course_id,
        course_slug,
        preview_lesson_id,
        locked_lesson_id,
    }
}

/// Pull `(content, video_url)` for a specific lesson out of the
/// `GET /api/courses/{slug}` response body.
fn lesson_body(body: &Value, lesson_id: Uuid) -> (&str, Option<&str>) {
    let modules = body["modules"].as_array().expect("modules array");
    for m in modules {
        let lessons = m["lessons"].as_array().expect("lessons array");
        for l in lessons {
            let id_str = l["id"].as_str().unwrap_or_default();
            if id_str == lesson_id.to_string() {
                let content = l["content"].as_str().unwrap_or_default();
                let video = l["video_url"].as_str();
                return (content, video);
            }
        }
    }
    panic!("lesson {} not found in course payload", lesson_id);
}

// ── Free course ──────────────────────────────────────────────────────────

#[tokio::test]
async fn free_course_lessons_are_fully_visible_to_anonymous() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let course = seed_course(&app, admin.id, true, false, 0).await;

    let resp = app
        .get(&format!("/api/courses/{}", course.course_slug), None)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("course body");

    let (preview_content, preview_video) = lesson_body(&body, course.preview_lesson_id);
    let (locked_content, locked_video) = lesson_body(&body, course.locked_lesson_id);

    assert_eq!(preview_content, "PREVIEW LESSON BODY");
    assert_eq!(preview_video, Some("https://cdn.example/preview.mp4"));
    // Free course: even the non-preview lesson is open.
    assert_eq!(locked_content, "PAID LESSON BODY");
    assert_eq!(locked_video, Some("https://cdn.example/locked.mp4"));
}

#[tokio::test]
async fn free_course_can_be_enrolled_without_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let course = seed_course(&app, admin.id, true, false, 0).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}

// ── Subscription-included course: redaction matrix ───────────────────────

#[tokio::test]
async fn anonymous_caller_sees_redacted_locked_lesson() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .get(&format!("/api/courses/{}", course.course_slug), None)
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");

    let (preview_content, preview_video) = lesson_body(&body, course.preview_lesson_id);
    let (locked_content, locked_video) = lesson_body(&body, course.locked_lesson_id);

    assert_eq!(preview_content, "PREVIEW LESSON BODY");
    assert_eq!(preview_video, Some("https://cdn.example/preview.mp4"));
    // Locked lesson must be empty body / no video for anon visitor.
    assert_eq!(locked_content, "");
    assert!(locked_video.is_none());
}

#[tokio::test]
async fn unsubscribed_member_sees_redacted_locked_lesson() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .get(
            &format!("/api/courses/{}", course.course_slug),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let (locked_content, locked_video) = lesson_body(&body, course.locked_lesson_id);
    assert_eq!(locked_content, "");
    assert!(locked_video.is_none());
}

#[tokio::test]
async fn active_subscriber_sees_full_locked_lesson() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_active_subscription(&app, &user).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .get(
            &format!("/api/courses/{}", course.course_slug),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let (locked_content, locked_video) = lesson_body(&body, course.locked_lesson_id);
    assert_eq!(locked_content, "PAID LESSON BODY");
    assert_eq!(locked_video, Some("https://cdn.example/locked.mp4"));
}

#[tokio::test]
async fn trialing_subscriber_sees_full_locked_lesson() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_subscription_with_status(&app, &user, SubscriptionStatus::Trialing).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .get(
            &format!("/api/courses/{}", course.course_slug),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let (locked_content, _) = lesson_body(&body, course.locked_lesson_id);
    assert_eq!(locked_content, "PAID LESSON BODY");
}

#[tokio::test]
async fn past_due_subscriber_does_not_see_locked_lesson() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_subscription_with_status(&app, &user, SubscriptionStatus::PastDue).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .get(
            &format!("/api/courses/{}", course.course_slug),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let (locked_content, locked_video) = lesson_body(&body, course.locked_lesson_id);
    assert_eq!(locked_content, "", "past_due must lose paid-content access");
    assert!(locked_video.is_none());
}

#[tokio::test]
async fn canceled_subscriber_does_not_see_locked_lesson() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_subscription_with_status(&app, &user, SubscriptionStatus::Canceled).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .get(
            &format!("/api/courses/{}", course.course_slug),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let (locked_content, _) = lesson_body(&body, course.locked_lesson_id);
    assert_eq!(locked_content, "");
}

#[tokio::test]
async fn admin_sees_full_locked_lesson_without_personal_subscription() {
    // QA / support need to view paid content — admins bypass the gate.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .get(
            &format!("/api/courses/{}", course.course_slug),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let (locked_content, _) = lesson_body(&body, course.locked_lesson_id);
    assert_eq!(locked_content, "PAID LESSON BODY");
}

// ── Subscription-included course: enrollment matrix ──────────────────────

#[tokio::test]
async fn enrollment_blocked_without_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn enrollment_succeeds_with_active_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_active_subscription(&app, &user).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn enrollment_succeeds_with_trial_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_subscription_with_status(&app, &user, SubscriptionStatus::Trialing).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn enrollment_blocked_with_past_due_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_subscription_with_status(&app, &user, SubscriptionStatus::PastDue).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn enrollment_blocked_with_canceled_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    give_subscription_with_status(&app, &user, SubscriptionStatus::Canceled).await;
    let course = seed_course(&app, admin.id, false, true, 0).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── Pay-per-course (no purchase ledger yet → blocked) ────────────────────

#[tokio::test]
async fn pay_per_course_enrollment_is_blocked_until_purchase_lands() {
    // `is_free=false`, `is_included_in_subscription=false`, price > 0
    // is the à-la-carte SKU. The purchase flow doesn't exist yet, so the
    // gate is closed: 403 — better than the pre-fix behaviour where any
    // authed user could enroll without paying.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let course = seed_course(&app, admin.id, false, false, 9900).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn pay_per_course_admin_can_enroll_for_qa() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let course = seed_course(&app, admin.id, false, false, 9900).await;

    let resp = app
        .post_json(
            &format!("/api/member/courses/{}/enroll", course.course_id),
            &serde_json::json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}

// ── Anonymous enrollment is always 401 ───────────────────────────────────

#[tokio::test]
async fn anonymous_enrollment_is_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let course = seed_course(&app, admin.id, true, false, 0).await;

    app.post_json(
        &format!("/api/member/courses/{}/enroll", course.course_id),
        &serde_json::json!({}),
        None,
    )
    .await
    .assert_status(StatusCode::UNAUTHORIZED);
}

// ── Unpublished course is invisible regardless of access ─────────────────

#[tokio::test]
async fn unpublished_course_returns_404_for_everyone() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let course_id = Uuid::new_v4();
    let suffix = &course_id.to_string()[..8];
    let course_slug = format!("draft-{}", suffix);
    sqlx::query(
        r#"
        INSERT INTO courses
            (id, title, slug, description, instructor_id, price_cents, currency,
             is_free, is_included_in_subscription, published)
        VALUES ($1, $2, $3, '', $4, 0, 'usd', TRUE, FALSE, FALSE)
        "#,
    )
    .bind(course_id)
    .bind(format!("Draft {}", suffix))
    .bind(&course_slug)
    .bind(admin.id)
    .execute(app.db())
    .await
    .expect("seed unpublished");

    app.get(&format!("/api/courses/{}", course_slug), None)
        .await
        .assert_status(StatusCode::NOT_FOUND);
}
