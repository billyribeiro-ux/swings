#![deny(warnings)]
#![forbid(unsafe_code)]

//! FDN-09 integration tests for the Resend [`EmailProvider`].
//!
//! These tests stand up a `wiremock` server and point the provider at it via
//! the `RESEND_API_BASE` override. They do not require the real Resend API
//! or any network egress and therefore run in the unit-test suite.

use swings_api::notifications::channels::email::ResendProvider;
use swings_api::notifications::{EmailProvider, EmailProviderError, EmailSendRequest};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn sample_request() -> EmailSendRequest {
    EmailSendRequest {
        to: "user@example.com".into(),
        from: "Swings <noreply@example.test>".into(),
        subject: "Welcome".into(),
        html_body: "<p>hi</p>".into(),
        plain_body: None,
        reply_to: Some("replies@example.test".into()),
        tags: vec![
            ("template".into(), "user.welcome".into()),
            ("locale".into(), "en".into()),
        ],
        idempotency_key: Some("idem-42".into()),
    }
}

async fn build_provider(base: &str) -> ResendProvider {
    ResendProvider::new(
        "re_test_key",
        "Swings <noreply@example.test>",
        base.to_string(),
    )
    .expect("builds")
}

#[tokio::test]
async fn send_happy_path_returns_provider_id() {
    let server = MockServer::start().await;

    let expected_body = serde_json::json!({
        "from": "Swings <noreply@example.test>",
        "to": ["user@example.com"],
        "subject": "Welcome",
        "html": "<p>hi</p>",
        "reply_to": "replies@example.test",
        "tags": [
            {"name": "template", "value": "user.welcome"},
            {"name": "locale", "value": "en"},
        ],
    });

    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(header("Authorization", "Bearer re_test_key"))
        .and(header("Idempotency-Key", "idem-42"))
        .and(body_json(&expected_body))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "re_abc_123"})),
        )
        .mount(&server)
        .await;

    let provider = build_provider(&server.uri()).await;
    let id = provider.send(&sample_request()).await.expect("send ok");
    assert_eq!(id, "re_abc_123");
    assert_eq!(provider.name(), "resend");
}

#[tokio::test]
async fn send_429_maps_to_transient() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .mount(&server)
        .await;

    let provider = build_provider(&server.uri()).await;
    let err = provider
        .send(&sample_request())
        .await
        .expect_err("transient");
    assert!(
        matches!(err, EmailProviderError::Transient(_)),
        "got: {err:?}"
    );
}

#[tokio::test]
async fn send_422_maps_to_permanent() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(422).set_body_json(
            serde_json::json!({"name": "validation_error", "message": "invalid address"}),
        ))
        .mount(&server)
        .await;

    let provider = build_provider(&server.uri()).await;
    let err = provider
        .send(&sample_request())
        .await
        .expect_err("permanent");
    assert!(
        matches!(err, EmailProviderError::Permanent(_)),
        "got: {err:?}"
    );
}

#[tokio::test]
async fn send_500_maps_to_transient() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(503).set_body_string("upstream hiccup"))
        .mount(&server)
        .await;

    let provider = build_provider(&server.uri()).await;
    let err = provider
        .send(&sample_request())
        .await
        .expect_err("transient");
    assert!(
        matches!(err, EmailProviderError::Transient(_)),
        "got: {err:?}"
    );
}

#[tokio::test]
async fn send_connection_error_maps_to_transient() {
    // Use a reserved unassigned port to guarantee the connection refuses.
    // 127.0.0.1:1 is outside the well-known range and rejects immediately.
    let provider = build_provider("http://127.0.0.1:1").await;
    let err = provider
        .send(&sample_request())
        .await
        .expect_err("transient");
    assert!(
        matches!(err, EmailProviderError::Transient(_)),
        "got: {err:?}"
    );
}

#[tokio::test]
async fn idempotency_key_header_only_sent_when_present() {
    let server = MockServer::start().await;

    // Request without idempotency key must NOT include the header.
    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(wiremock::matchers::header_exists("authorization"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "re_no_idem"})),
        )
        .mount(&server)
        .await;

    let provider = build_provider(&server.uri()).await;
    let mut req = sample_request();
    req.idempotency_key = None;
    let id = provider.send(&req).await.expect("send ok");
    assert_eq!(id, "re_no_idem");

    // Sanity: the request we just sent did not carry Idempotency-Key.
    let received = server.received_requests().await.expect("received");
    let last: &Request = received
        .iter()
        .find(|r| r.url.path() == "/emails")
        .expect("got the request");
    assert!(
        last.headers.get("Idempotency-Key").is_none(),
        "Idempotency-Key header leaked when not requested"
    );
}
