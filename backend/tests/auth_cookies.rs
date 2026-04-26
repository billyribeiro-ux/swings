#![deny(warnings)]
#![forbid(unsafe_code)]

//! BFF auth-cookie integration coverage (Phase 1.3 of `docs/REMAINING-WORK.md`).
//!
//! These tests pin the contract that the migration from
//! `localStorage`-backed bearer tokens to httpOnly `Set-Cookie` -delivered
//! sessions does not silently regress:
//!
//! 1. `POST /api/auth/login` mints both `swings_access` and `swings_refresh`
//!    cookies, both `HttpOnly`, both scoped to `/`.
//! 2. The cookie alone is enough to authenticate `GET /api/auth/me` —
//!    bearer header is no longer required.
//! 3. The bearer header still works for clients on the legacy path
//!    (Phase A backwards compatibility).
//! 4. `POST /api/auth/logout` emits deletion cookies (`Max-Age=0`) for both
//!    halves.
//! 5. After the deletion cookies have been swallowed, the now-empty cookie
//!    must not authenticate.
//! 6. `Secure` flag toggles with `Config::is_production()`.
//!
//! Skipped when no test database is configured — same convention as the rest
//! of `backend/tests/`.

mod support;

use axum::http::StatusCode;
use serde_json::json;
use support::TestApp;

/// Pull the names of every cookie minted by the response. Used to assert the
/// expected `swings_access` / `swings_refresh` pair without coupling tests to
/// the exact attribute order or casing.
fn cookie_names(resp_headers: &axum::http::HeaderMap) -> Vec<String> {
    resp_headers
        .get_all(axum::http::header::SET_COOKIE)
        .into_iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|s| s.split('=').next().map(str::trim).map(str::to_owned))
        .collect()
}

/// Return the raw `Set-Cookie` line for `name`, panicking with a descriptive
/// message if the header is absent. We compare on lowercase prefix so an
/// `HttpOnly` re-spelling does not break the assertion.
fn set_cookie_for<'h>(headers: &'h axum::http::HeaderMap, name: &str) -> &'h str {
    headers
        .get_all(axum::http::header::SET_COOKIE)
        .into_iter()
        .filter_map(|v| v.to_str().ok())
        .find(|s| s.starts_with(&format!("{name}=")))
        .unwrap_or_else(|| {
            panic!(
                "expected Set-Cookie for {name}, got {:?}",
                cookie_names(headers)
            )
        })
}

/// Extract the `value` portion from a `name=value; Attr1; Attr2` header.
fn cookie_value<'a>(set_cookie_line: &'a str, name: &str) -> &'a str {
    let prefix = format!("{name}=");
    let rest = set_cookie_line
        .strip_prefix(&prefix)
        .expect("Set-Cookie line missing the expected name= prefix");
    // Up to the first attribute boundary `;` is the value.
    rest.split(';').next().unwrap_or(rest)
}

#[tokio::test]
async fn login_sets_access_and_refresh_cookies_with_httponly() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": user.password }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let names = cookie_names(resp.headers());
    assert!(
        names.iter().any(|n| n == "swings_access"),
        "expected swings_access cookie; got {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "swings_refresh"),
        "expected swings_refresh cookie; got {names:?}"
    );

    // HttpOnly defeats the XSS-exfiltration class — it MUST be present on
    // both halves. Path=/ keeps the cookies live across both /api/* and
    // any future page-served route.
    let access_line = set_cookie_for(resp.headers(), "swings_access");
    assert!(
        access_line.to_ascii_lowercase().contains("httponly"),
        "swings_access cookie missing HttpOnly: {access_line}"
    );
    assert!(
        access_line.to_ascii_lowercase().contains("path=/"),
        "swings_access cookie missing Path=/: {access_line}"
    );
    assert!(
        access_line.to_ascii_lowercase().contains("samesite=lax"),
        "swings_access cookie missing SameSite=Lax: {access_line}"
    );
    assert!(
        access_line.to_ascii_lowercase().contains("max-age="),
        "swings_access cookie missing Max-Age: {access_line}"
    );

    let refresh_line = set_cookie_for(resp.headers(), "swings_refresh");
    assert!(
        refresh_line.to_ascii_lowercase().contains("httponly"),
        "swings_refresh cookie missing HttpOnly: {refresh_line}"
    );

    // The test harness runs in `app_env=test` (NOT production), so neither
    // cookie should carry the `Secure` flag — that would prevent the dev
    // path from working over `http://localhost`.
    assert!(
        !access_line.to_ascii_lowercase().contains("secure"),
        "swings_access cookie should NOT be Secure under app_env=test: {access_line}"
    );
    assert!(
        !refresh_line.to_ascii_lowercase().contains("secure"),
        "swings_refresh cookie should NOT be Secure under app_env=test: {refresh_line}"
    );

    // The cookie body MUST be the JSON access_token — confirms a single
    // source of truth between the two carriers during Phase A. Phase B will
    // drop the JSON body and tighten this assertion to "cookie only".
    let body: serde_json::Value = resp.json().expect("login JSON body");
    let json_access = body["access_token"].as_str().expect("access_token in body");
    let cookie_access = cookie_value(access_line, "swings_access");
    assert_eq!(
        json_access, cookie_access,
        "JSON access_token and cookie value must agree during Phase A"
    );
}

#[tokio::test]
async fn me_authenticates_via_cookie_only() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": user.password }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let access_line = set_cookie_for(resp.headers(), "swings_access");
    let access_value = cookie_value(access_line, "swings_access");

    // Hand-craft a request that carries ONLY the cookie — no Authorization
    // header. This is the post-rollout posture from the SPA.
    let me = app
        .request_with_cookie("GET", "/api/auth/me", "swings_access", access_value)
        .await;
    me.assert_status(StatusCode::OK);
    let body: serde_json::Value = me.json().expect("me JSON");
    assert_eq!(body["email"].as_str(), Some(user.email.as_str()));
}

#[tokio::test]
async fn me_still_authenticates_via_bearer_for_backwards_compat() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    // Use the harness-minted bearer token directly — no cookie header at
    // all. Mirrors a long-lived legacy SPA tab during the rollout window.
    let me = app.get("/api/auth/me", Some(&user.access_token)).await;
    me.assert_status(StatusCode::OK);
    let body: serde_json::Value = me.json().expect("me JSON");
    assert_eq!(body["email"].as_str(), Some(user.email.as_str()));
}

#[tokio::test]
async fn logout_clears_both_session_cookies() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    // Logout uses the AuthUser extractor — present a bearer header so the
    // harness can drive it without round-tripping login first. The cookie
    // path is exercised by `me_authenticates_via_cookie_only`.
    let resp = app
        .post_json("/api/auth/logout", &json!({}), Some(&user.access_token))
        .await;
    resp.assert_status(StatusCode::OK);

    let access_line = set_cookie_for(resp.headers(), "swings_access");
    let refresh_line = set_cookie_for(resp.headers(), "swings_refresh");

    // Deletion cookies are spelled `Max-Age=0` (the value remains empty).
    assert!(
        access_line.to_ascii_lowercase().contains("max-age=0"),
        "logout swings_access should be Max-Age=0: {access_line}"
    );
    assert!(
        refresh_line.to_ascii_lowercase().contains("max-age=0"),
        "logout swings_refresh should be Max-Age=0: {refresh_line}"
    );

    // The empty-value cookie MUST NOT authenticate when re-presented.
    // (Browsers would already swallow this server hint and drop the row,
    // but a non-conformant client that resends it should still 401.)
    let me = app
        .request_with_cookie("GET", "/api/auth/me", "swings_access", "")
        .await;
    me.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cookie_value_overrides_bearer_when_both_present() {
    // Defense-in-depth: during the rollout some requests will momentarily
    // carry BOTH the new cookie AND the old bearer header. The cookie MUST
    // win — otherwise a stale localStorage token could outlive a server
    // logout that cleared the cookie.
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    // Hand-craft: cookie is junk, bearer is valid → request should 401
    // (cookie wins, then fails to decode).
    let resp = app
        .request_with_cookie_and_bearer(
            "GET",
            "/api/auth/me",
            "swings_access",
            "this-is-not-a-jwt",
            &user.access_token,
        )
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// Cookie-attribute coverage for production posture — verified via a focused
// unit test in `backend/src/handlers/auth.rs::tests` (see
// `secure_cookie_attribute_in_production`). Driving this via `TestApp` would
// require swapping out `app_env`, which the harness intentionally does not
// expose (the `Config` builder is private to `support::app`).
