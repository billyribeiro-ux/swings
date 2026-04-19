#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-18 integration coverage for the per-actor admin-mutation
//! rate-limit. Uses the in-process backend (default in tests) so the
//! `governor::keyed::RateLimiter` actually decides the verdict.
//!
//! Properties asserted:
//!   * `GET` requests are exempt — operators can keep dashboards live
//!     even when their mutation budget is exhausted.
//!   * Two distinct admins are bucketed independently (per-actor
//!     keying, not per-IP).
//!   * Burst exhaustion produces `429 Too Many Requests`.
//!
//! We reuse a low-side-effect endpoint — `POST /api/admin/dsar/jobs/cancel/{id}`
//! returning `404` for an unknown id — so each request is cheap and
//! deterministic without persisting fixtures. The middleware sits
//! above route resolution, so a 404 from the inner handler still
//! counts toward the bucket.

mod support;

use axum::http::StatusCode;
use serde_json::json;
use support::TestApp;

const FAKE_JOB: &str = "/api/admin/dsar/jobs/00000000-0000-0000-0000-000000000000/cancel";
const QUOTA: u32 = 240; // mirrors `ADMIN_MUTATION.max_requests`

#[tokio::test]
async fn get_requests_are_exempt_from_admin_mutation_quota() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    // 300 GETs — well above the mutation quota — must all succeed
    // (or 200/404 from the handler) without a single 429.
    for _ in 0..(QUOTA + 60) {
        let resp = app
            .get("/api/admin/dsar/jobs", Some(&admin.access_token))
            .await;
        assert_ne!(
            resp.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "GET dashboards must never be rate-limited"
        );
    }
}

#[tokio::test]
async fn admin_mutations_burst_eventually_429s() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    let mut saw_429 = false;
    // Try 2x the quota; the keyed governor's burst is `QUOTA` so we
    // are guaranteed to cross the cap inside this loop. We break on
    // the first 429 to keep the test fast and avoid runaway logs.
    for i in 0..(QUOTA * 2) {
        let resp = app
            .post_json::<serde_json::Value>(
                FAKE_JOB,
                &json!({ "reason": "ratelimit-probe" }),
                Some(&admin.access_token),
            )
            .await;
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            assert!(
                i >= QUOTA / 2,
                "rate-limit fired too aggressively at i={i} (quota={QUOTA})"
            );
            saw_429 = true;
            break;
        }
    }
    assert!(
        saw_429,
        "expected at least one 429 inside {} mutations",
        QUOTA * 2
    );
}

#[tokio::test]
async fn distinct_admins_get_independent_buckets() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_admin().await.expect("alice");
    let bob = app.seed_admin().await.expect("bob");
    assert_ne!(alice.id, bob.id);

    // Burn most of alice's bucket.
    for _ in 0..(QUOTA - 20) {
        let _ = app
            .post_json::<serde_json::Value>(
                FAKE_JOB,
                &json!({ "reason": "alice" }),
                Some(&alice.access_token),
            )
            .await;
    }

    // Bob, with a fresh JWT sub, must still be allowed without
    // 429ing — proving the limiter keys on actor, not IP.
    let bob_attempts = 50;
    for _ in 0..bob_attempts {
        let resp = app
            .post_json::<serde_json::Value>(
                FAKE_JOB,
                &json!({ "reason": "bob" }),
                Some(&bob.access_token),
            )
            .await;
        assert_ne!(
            resp.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "bob's bucket must be independent of alice's"
        );
    }
}

#[tokio::test]
async fn unauthenticated_mutations_are_rejected_before_rate_limit() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    // No bearer → upstream 401 from the auth extractor; the rate-
    // limit middleware should still pass the request through (it has
    // no actor to charge, and the auth layer rejects it on its own).
    // Even repeated probes must not be 429ed because the bucket key
    // would be IP-only and we don't want a malicious unauth client to
    // poison the bucket of legitimate admins behind the same egress.
    for _ in 0..30 {
        let resp = app
            .post_json::<serde_json::Value>(
                FAKE_JOB,
                &json!({ "reason": "anon" }),
                None,
            )
            .await;
        assert!(
            matches!(
                resp.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ),
            "got unexpected {} for unauthenticated mutation",
            resp.status()
        );
    }
}
