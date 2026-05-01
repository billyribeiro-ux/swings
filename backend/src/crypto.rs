use sha2::{Digest, Sha256};

/// SHA-256 hex digest of `token`. Used by auth handlers to store and look up
/// single-use tokens (email verification, password reset, refresh tokens)
/// without persisting the raw secret.
///
/// `pub` (not `pub(crate)`) so integration tests in `backend/tests/` can
/// reproduce the hash when seeding token rows directly into the DB.
#[doc(hidden)]
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
