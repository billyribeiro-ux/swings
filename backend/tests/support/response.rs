//! Typed wrapper around [`axum::response::Response`] for ergonomic assertions.
//!
//! Integration tests often spell out the same boilerplate — read the body,
//! parse it, deserialize, pretty-print on failure. [`TestResponse`] funnels
//! all of that into a single type that retains both the HTTP status/headers
//! and the raw body bytes, so failure messages can include the server's
//! actual response payload without an extra round-trip.

use axum::{
    body::to_bytes,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use serde::de::DeserializeOwned;

use super::error::{TestAppError, TestResult};

/// Maximum response body size the harness will buffer into memory (4 MiB).
///
/// In-process `oneshot` responses are fully buffered by design; 4 MiB is
/// enough to cover every JSON payload the API produces while preventing a
/// runaway handler from exhausting test-runner memory.
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// Captured response from a [`super::TestApp`] HTTP call.
///
/// Holds the status code + headers from the live response plus the body
/// bytes fully drained into memory. The raw `Response` is intentionally
/// consumed so tests cannot accidentally dispatch it a second time.
#[derive(Debug)]
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl TestResponse {
    pub(crate) async fn from_response(resp: Response) -> TestResult<Self> {
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp.into_body();
        let bytes = to_bytes(body, MAX_BODY_BYTES)
            .await
            .map_err(|e| TestAppError::Http(format!("read response body: {e}")))?;
        Ok(Self {
            status,
            headers,
            body: bytes.to_vec(),
        })
    }

    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns the named header's value if present *and* UTF-8.
    ///
    /// Values with non-ASCII characters (which the API shouldn't produce)
    /// are reported as absent rather than panicking the test harness.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    #[must_use]
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    /// Best-effort UTF-8 decoding of the response body.
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// Deserialize the response body as JSON.
    ///
    /// Errors carry the raw body text in their message so the test log shows
    /// exactly what the handler returned when the schema mismatches.
    pub fn json<T: DeserializeOwned>(&self) -> TestResult<T> {
        serde_json::from_slice(&self.body).map_err(|e| {
            TestAppError::Decode(format!(
                "deserialize JSON ({e}); body={}",
                String::from_utf8_lossy(&self.body)
            ))
        })
    }

    /// Parse the response as an RFC 7807 Problem body (`application/problem+json`).
    ///
    /// Returns the fields the harness asserts against; unknown fields are
    /// ignored so schema additions elsewhere don't force every test to update.
    pub fn problem(&self) -> TestResult<ProblemBody> {
        self.json::<ProblemBody>()
    }

    /// Assert the status matches `expected`; on failure, the panic message
    /// includes both the actual status and the response body to speed up
    /// triage. Use `assert_status` in place of `assert_eq!(resp.status(),…)`
    /// unless you have a reason not to.
    #[track_caller]
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        if self.status != expected {
            panic!(
                "expected status {expected}, got {}. body=\n{}",
                self.status,
                self.text()
            );
        }
        self
    }

    /// Assert the body is an RFC 7807 Problem with the expected `status`,
    /// `type` suffix, and `title` fields.
    ///
    /// `type_suffix` is matched against the portion of `type` after the
    /// `/problems/` prefix that `error.rs` uses, so callers do not need to
    /// hard-code the full URI.
    #[track_caller]
    pub fn assert_problem(&self, expected: AssertProblem) -> &Self {
        self.assert_status(expected.status);
        let ct = self.header("content-type").unwrap_or_default();
        if !ct.starts_with("application/problem+json") {
            panic!(
                "expected content-type application/problem+json, got {ct:?}. body=\n{}",
                self.text()
            );
        }
        let body = match self.problem() {
            Ok(b) => b,
            Err(e) => panic!("problem body was not JSON: {e}. body=\n{}", self.text()),
        };
        let expected_type = format!("/problems/{}", expected.type_suffix);
        if body.type_uri != expected_type {
            panic!(
                "expected problem type {expected_type:?}, got {:?}. body=\n{}",
                body.type_uri,
                self.text()
            );
        }
        if body.title != expected.title {
            panic!(
                "expected problem title {:?}, got {:?}. body=\n{}",
                expected.title,
                body.title,
                self.text()
            );
        }
        if body.status != expected.status.as_u16() {
            panic!(
                "expected problem status {}, got {}. body=\n{}",
                expected.status.as_u16(),
                body.status,
                self.text()
            );
        }
        self
    }
}

/// Expected fields of an RFC 7807 Problem response.
///
/// Populate and pass to [`TestResponse::assert_problem`]; `detail` is
/// deliberately not asserted on by default because it is often dynamic
/// user-facing text — callers who need to check it should use
/// `resp.problem()?.detail`.
#[derive(Debug, Clone)]
pub struct AssertProblem<'a> {
    pub status: StatusCode,
    pub type_suffix: &'a str,
    pub title: &'a str,
}

/// Loose deserialization of an RFC 7807 Problem body.
///
/// Extra fields on the Problem (`instance`, `correlation_id`, …) are
/// permitted and ignored so tests stay stable across orthogonal additions.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProblemBody {
    #[serde(rename = "type")]
    pub type_uri: String,
    pub title: String,
    pub status: u16,
    #[serde(default)]
    pub detail: String,
}
