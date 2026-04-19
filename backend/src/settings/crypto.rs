//! AES-256-GCM envelope used by `value_type = 'secret'` rows.
//!
//! The settings catalogue cannot reuse `forms::integration_config`
//! because that module is keyed on `APP_DATA_KEY` (form integrations)
//! and its envelope shape (`{ciphertext, nonce}`) is internal to the
//! forms feature. Settings has a dedicated key
//! (`SETTINGS_ENCRYPTION_KEY`) so a leaked form key cannot be used to
//! decrypt admin settings, and ships a versioned envelope so we can
//! rotate the AEAD construction in the future without a destructive
//! migration.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("SETTINGS_ENCRYPTION_KEY env var not set")]
    KeyMissing,
    #[error("SETTINGS_ENCRYPTION_KEY is not a valid base64 32-byte key")]
    KeyInvalid,
    #[error("ciphertext base64 decode failed")]
    BadCiphertextEncoding,
    #[error("nonce base64 decode failed or wrong length")]
    BadNonce,
    #[error("envelope version unsupported: {0}")]
    UnsupportedVersion(u8),
    #[error("AES-GCM decryption failed (wrong key or tampered ciphertext)")]
    DecryptFailed,
    #[error("AES-GCM encryption failed")]
    EncryptFailed,
}

const ENVELOPE_V1: u8 = 1;

/// On-disk shape for a sealed setting. Stored as a JSONB object on
/// the row's `value` column; the version tag lets us roll the AEAD or
/// switch to wrapped DEKs in a later migration without rewriting all
/// historical envelopes at once.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    /// Ciphertext (AES-GCM output), base64.
    pub ct: String,
    /// 12-byte nonce, base64.
    pub nonce: String,
    /// Envelope schema version. Currently always 1.
    pub v: u8,
}

fn key_from_env_or(key_b64: &str) -> Result<Key<Aes256Gcm>, CryptoError> {
    let bytes = B64.decode(key_b64).map_err(|_| CryptoError::KeyInvalid)?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::KeyInvalid)?;
    Ok(Key::<Aes256Gcm>::from(arr))
}

/// Read the runtime key from `SETTINGS_ENCRYPTION_KEY`. Missing →
/// `KeyMissing` so the admin handler can surface a 503 with an
/// operator-actionable message.
pub fn key_from_env() -> Result<String, CryptoError> {
    std::env::var("SETTINGS_ENCRYPTION_KEY").map_err(|_| CryptoError::KeyMissing)
}

pub fn encrypt(plaintext: &str, key_b64: &str) -> Result<Envelope, CryptoError> {
    let key = key_from_env_or(key_b64)?;
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::EncryptFailed)?;
    Ok(Envelope {
        ct: B64.encode(ct),
        nonce: B64.encode(nonce),
        v: ENVELOPE_V1,
    })
}

pub fn decrypt(envelope: &Envelope, key_b64: &str) -> Result<String, CryptoError> {
    if envelope.v != ENVELOPE_V1 {
        return Err(CryptoError::UnsupportedVersion(envelope.v));
    }
    let key = key_from_env_or(key_b64)?;
    let cipher = Aes256Gcm::new(&key);
    let ct = B64
        .decode(&envelope.ct)
        .map_err(|_| CryptoError::BadCiphertextEncoding)?;
    let nonce_bytes: [u8; 12] = B64
        .decode(&envelope.nonce)
        .map_err(|_| CryptoError::BadNonce)?
        .try_into()
        .map_err(|_| CryptoError::BadNonce)?;
    let nonce = Nonce::from(nonce_bytes);
    let pt = cipher
        .decrypt(&nonce, ct.as_slice())
        .map_err(|_| CryptoError::DecryptFailed)?;
    String::from_utf8(pt).map_err(|_| CryptoError::DecryptFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> String {
        B64.encode([7u8; 32])
    }

    #[test]
    fn round_trip() {
        let key = test_key();
        let env = encrypt("super-sekrit", &key).unwrap();
        assert_eq!(env.v, 1);
        let pt = decrypt(&env, &key).unwrap();
        assert_eq!(pt, "super-sekrit");
    }

    #[test]
    fn wrong_key_fails() {
        let env = encrypt("x", &test_key()).unwrap();
        let bad = B64.encode([9u8; 32]);
        assert!(matches!(decrypt(&env, &bad), Err(CryptoError::DecryptFailed)));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = test_key();
        let mut env = encrypt("hello", &key).unwrap();
        let mut bytes = B64.decode(&env.ct).unwrap();
        bytes[0] ^= 0x01;
        env.ct = B64.encode(&bytes);
        assert!(matches!(decrypt(&env, &key), Err(CryptoError::DecryptFailed)));
    }

    #[test]
    fn unsupported_version_rejected() {
        let mut env = encrypt("x", &test_key()).unwrap();
        env.v = 99;
        assert!(matches!(
            decrypt(&env, &test_key()),
            Err(CryptoError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn invalid_key_b64_rejected() {
        let env = encrypt("x", &test_key()).unwrap();
        assert!(matches!(decrypt(&env, "!!!not-base64!!!"), Err(CryptoError::KeyInvalid)));
    }
}
