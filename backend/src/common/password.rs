//! Argon2 password hashing + verification, off-loaded onto the blocking
//! thread pool.
//!
//! Forensic Wave-3 PR (W3-2): the original handlers called
//! `Argon2::default().hash_password(...)` and `.verify_password(...)`
//! directly inside `async fn` request handlers. With the default Argon2
//! parameters (m=19_456 KiB, t=2, p=1) each call burns 50–100 ms of CPU
//! on a modern laptop and ≥ 200 ms on small Railway instances — long
//! enough to starve the Tokio worker thread the handler is running on,
//! preventing it from polling other futures.
//!
//! The fix is `tokio::task::spawn_blocking`. The blocking pool is sized
//! independently of the worker pool (default 512 threads), so the
//! computation no longer occupies a worker. The two helpers below own
//! the salt generation + parsed-hash construction so call sites stay a
//! single `.await` on a single `Result`.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString};
use argon2::{Argon2, PasswordVerifier};

use crate::error::{AppError, AppResult};

/// Hash a plaintext password with Argon2 and return the encoded
/// `$argon2id$...` string.
///
/// Runs the hash on the Tokio blocking pool so the request worker
/// thread is never stalled on the ~100 ms CPU step. Errors from the
/// hasher are surfaced as [`AppError::BadRequest`] with the underlying
/// reason — the only failure mode in practice is a malformed password
/// length the caller already validated, so this branch is rare.
pub async fn hash_password(plaintext: String) -> AppResult<String> {
    let result = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(plaintext.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| format!("password hash error: {e}"))
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 join failed: {e}")))?;

    result.map_err(AppError::BadRequest)
}

/// Verify a plaintext password against a previously-stored Argon2
/// encoded hash. Returns `Ok(true)` when the password matches,
/// `Ok(false)` when it does not, and [`AppError`] only when the stored
/// hash is malformed (which would indicate a bug, not a wrong
/// password).
///
/// Like [`hash_password`], the actual Argon2 work runs on the blocking
/// pool. The stored hash and plaintext are moved into the closure so
/// no borrow leaks across the await point.
pub async fn verify_password(plaintext: String, stored_hash: String) -> AppResult<bool> {
    let outcome = tokio::task::spawn_blocking(move || {
        let parsed = match PasswordHash::new(&stored_hash) {
            Ok(p) => p,
            Err(e) => return Err(format!("invalid stored password hash: {e}")),
        };
        Ok(Argon2::default()
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("argon2 join failed: {e}")))?;

    outcome.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

/// Run the dummy Argon2 verify used for login-timing equalisation on
/// the "user not found" branch, so attackers cannot distinguish
/// non-existent accounts via response latency.
///
/// Hot path: this fires on every failed login. The blocking pool is
/// the right home — same rationale as [`hash_password`]. The function
/// returns `()` because the result is intentionally discarded.
pub async fn run_timing_equaliser_verify() {
    let _ = tokio::task::spawn_blocking(|| {
        // Generate a one-shot dummy hash so the verify path runs against
        // a real `PasswordHash` shape rather than a constant. The
        // hash result is dropped; only the wall time matters.
        let salt = SaltString::generate(&mut OsRng);
        if let Ok(dummy_hash) = Argon2::default().hash_password(b"timing-equalisation-dummy", &salt)
        {
            let serialized = dummy_hash.to_string();
            if let Ok(parsed) = PasswordHash::new(&serialized) {
                let _ = Argon2::default().verify_password(b"not-a-real-password", &parsed);
            }
        }
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_then_verify_round_trip() {
        let pw = "correct horse battery staple".to_string();
        let h = hash_password(pw.clone()).await.expect("hash ok");
        assert!(verify_password(pw, h).await.expect("verify ok"));
    }

    #[tokio::test]
    async fn wrong_password_returns_false_not_error() {
        let h = hash_password("right".to_string()).await.expect("hash ok");
        assert!(!verify_password("wrong".to_string(), h)
            .await
            .expect("verify ok"));
    }

    #[tokio::test]
    async fn malformed_stored_hash_is_internal_error() {
        let res = verify_password("pw".to_string(), "not-a-hash".to_string()).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn timing_equaliser_does_not_panic() {
        // Fires the same blocking-pool path as a real verify but
        // discards the outcome.
        run_timing_equaliser_verify().await;
    }
}
