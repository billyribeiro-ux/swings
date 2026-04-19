//! FORM-07: per-form integration configuration with envelope-encrypted
//! credentials.
//!
//! # Threat model & design
//!
//! Each form's `settings.integrations` JSONB column stores a list of
//! [`IntegrationConfig`] entries. Secret material (API keys, OAuth refresh
//! tokens, service-account JWT private keys) is sealed with AES-256-GCM
//! using a 32-byte key and persisted as `(base64 ciphertext, base64 nonce)`.
//! Plaintext only ever exists in memory between an [`unseal`][SealedCredential::unseal]
//! call and the integration's outbound HTTP request.
//!
//! # Why a `Cipher` value type instead of reading `APP_DATA_KEY` per call
//!
//! The original implementation called `std::env::var("APP_DATA_KEY")` on
//! every `encrypt` / `decrypt`. That is ambient process-global state — two
//! units of work racing on `std::env::{set_var,remove_var}` are undefined
//! behaviour (this is why Rust 2024 marks those functions `unsafe`). Tests
//! that wanted to install a fixture key had no way to do so without either
//! (a) mutating the env, which raced with every other thread reading env
//! (tokio runtime probing `RUST_LOG`, panic handlers reading
//! `RUST_BACKTRACE`, etc.), or (b) taking a module-local mutex, which
//! only serialises *this crate's* tests and does nothing for the runtime
//! machinery also calling `getenv(3)` concurrently.
//!
//! The fix is dependency injection:
//!
//! * [`Cipher`] owns the key bytes and exposes `encrypt` / `decrypt` as
//!   pure methods. Construct one via [`Cipher::from_env`] (production
//!   startup) or [`Cipher::from_key_bytes`] (tests, future key-rotation).
//! * A process-lifetime [`OnceLock<Cipher>`](std::sync::OnceLock) lazily
//!   resolves the env-backed cipher exactly once. Subsequent calls hit a
//!   pointer read; no env access, no allocation.
//! * The ergonomic top-level helpers ([`encrypt`], [`decrypt`],
//!   [`SealedCredential::seal`], [`SealedCredential::unseal`]) dispatch
//!   through the `OnceLock`. Production callers are unchanged.
//! * Tests construct a `Cipher` with deterministic bytes and use the
//!   explicit variants ([`SealedCredential::seal_with`],
//!   [`SealedCredential::unseal_with`]). **No test ever mutates the
//!   environment.**

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("APP_DATA_KEY env var not set")]
    KeyMissing,
    #[error("APP_DATA_KEY is not a valid base64 32-byte key")]
    KeyInvalid,
    #[error("ciphertext base64 decode failed")]
    BadCiphertextEncoding,
    #[error("nonce base64 decode failed or wrong length")]
    BadNonce,
    #[error("AES-GCM decryption failed (wrong key or tampered ciphertext)")]
    DecryptFailed,
    #[error("AES-GCM encryption failed")]
    EncryptFailed,
}

/// Owns a 32-byte AES-256-GCM key and the cipher primitives built from it.
///
/// Construction is infallible once you have valid key bytes; the fallible
/// paths are [`Cipher::from_env`] (env var must be set + valid base64 +
/// exactly 32 bytes) and [`Cipher::from_base64`] (same checks, no env).
/// Prefer passing `&Cipher` around over reaching for ambient helpers —
/// that makes every call site explicit about which key material it uses.
#[derive(Clone)]
pub struct Cipher {
    key: Key<Aes256Gcm>,
}

impl std::fmt::Debug for Cipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never leak key material via Debug, even in test logs.
        f.debug_struct("Cipher")
            .field("key", &"[redacted]")
            .finish()
    }
}

impl Cipher {
    /// Read `APP_DATA_KEY` (base64 of 32 raw bytes) from the process
    /// environment and build a cipher. Called at most once per process
    /// in production; see [`process_cipher`].
    pub fn from_env() -> Result<Self, CryptoError> {
        let raw = std::env::var("APP_DATA_KEY").map_err(|_| CryptoError::KeyMissing)?;
        Self::from_base64(&raw)
    }

    /// Build a cipher from a base64-encoded 32-byte key. Useful when the
    /// key lives in a secrets manager string other than `APP_DATA_KEY`.
    pub fn from_base64(b64_key: &str) -> Result<Self, CryptoError> {
        let bytes = B64
            .decode(b64_key.trim())
            .map_err(|_| CryptoError::KeyInvalid)?;
        let arr: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::KeyInvalid)?;
        Ok(Self::from_key_bytes(arr))
    }

    /// Build a cipher from raw key bytes. The canonical entry point for
    /// tests and for any legitimate caller that resolves its own key
    /// material (rotation, multi-tenant sharding, HSM-delivered bytes).
    pub fn from_key_bytes(bytes: [u8; 32]) -> Self {
        Self {
            key: Key::<Aes256Gcm>::from(bytes),
        }
    }

    /// Encrypt `plaintext` with a freshly generated 12-byte nonce.
    /// Returns `(ciphertext_b64, nonce_b64)`.
    pub fn encrypt(&self, plaintext: &str) -> Result<(String, String), CryptoError> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::EncryptFailed)?;
        Ok((B64.encode(ct), B64.encode(nonce)))
    }

    /// Decrypt a previously-[`Cipher::encrypt`]-ed pair. The nonce MUST
    /// have come from the matching encrypt call; AES-GCM requires unique
    /// nonces per key, but reuse only degrades confidentiality — the
    /// authentication tag still rejects tampering.
    pub fn decrypt(&self, ciphertext_b64: &str, nonce_b64: &str) -> Result<String, CryptoError> {
        let cipher = Aes256Gcm::new(&self.key);
        let ct = B64
            .decode(ciphertext_b64)
            .map_err(|_| CryptoError::BadCiphertextEncoding)?;
        let nonce_bytes: [u8; 12] = B64
            .decode(nonce_b64)
            .map_err(|_| CryptoError::BadNonce)?
            .try_into()
            .map_err(|_| CryptoError::BadNonce)?;
        let nonce = Nonce::from(nonce_bytes);
        let pt = cipher
            .decrypt(&nonce, ct.as_slice())
            .map_err(|_| CryptoError::DecryptFailed)?;
        String::from_utf8(pt).map_err(|_| CryptoError::DecryptFailed)
    }
}

/// Process-lifetime singleton, resolved exactly once from `APP_DATA_KEY`.
/// `OnceLock` stores only the *successful* cipher; repeat failures
/// continue to surface `KeyMissing` / `KeyInvalid` on each call so
/// operator mis-configuration does not silently persist.
static PROCESS_CIPHER: OnceLock<Cipher> = OnceLock::new();

fn process_cipher() -> Result<&'static Cipher, CryptoError> {
    if let Some(c) = PROCESS_CIPHER.get() {
        return Ok(c);
    }
    let fresh = Cipher::from_env()?;
    Ok(PROCESS_CIPHER.get_or_init(|| fresh))
}

/// Ergonomic top-level shim over [`Cipher::encrypt`] using the
/// process-wide env-resolved key. Prefer holding a [`Cipher`] value when
/// you have one (tests, rotation); this exists for call sites that
/// previously depended on the free function.
pub fn encrypt(plaintext: &str) -> Result<(String, String), CryptoError> {
    process_cipher()?.encrypt(plaintext)
}

/// Ergonomic top-level shim over [`Cipher::decrypt`]. See [`encrypt`].
pub fn decrypt(ciphertext_b64: &str, nonce_b64: &str) -> Result<String, CryptoError> {
    process_cipher()?.decrypt(ciphertext_b64, nonce_b64)
}

/// Encrypted-at-rest credential bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedCredential {
    pub ciphertext: String,
    pub nonce: String,
}

impl SealedCredential {
    /// Seal using the process-wide env-resolved key. Production path.
    pub fn seal(plaintext: &str) -> Result<Self, CryptoError> {
        Self::seal_with(process_cipher()?, plaintext)
    }

    /// Unseal using the process-wide env-resolved key. Production path.
    pub fn unseal(&self) -> Result<String, CryptoError> {
        self.unseal_with(process_cipher()?)
    }

    /// Seal with an explicit [`Cipher`]. **Pure** — no env access, no
    /// OnceLock mutation. The path tests, key-rotation code, and
    /// multi-tenant callers should use.
    pub fn seal_with(cipher: &Cipher, plaintext: &str) -> Result<Self, CryptoError> {
        let (ct, n) = cipher.encrypt(plaintext)?;
        Ok(Self {
            ciphertext: ct,
            nonce: n,
        })
    }

    /// Unseal with an explicit [`Cipher`]. Pure; see [`Self::seal_with`].
    pub fn unseal_with(&self, cipher: &Cipher) -> Result<String, CryptoError> {
        cipher.decrypt(&self.ciphertext, &self.nonce)
    }
}

/// One row of `forms.settings.integrations`. The provider tag drives the
/// adapter dispatch in [`crate::events::handlers::integrations`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum IntegrationConfig {
    Mailchimp {
        list_id: String,
        api_key: SealedCredential,
    },
    ActiveCampaign {
        account_url: String,
        api_key: SealedCredential,
    },
    ConvertKit {
        form_id: String,
        api_key: SealedCredential,
    },
    HubSpot {
        access_token: SealedCredential,
        list_id: Option<String>,
    },
    Salesforce {
        instance_url: String,
        access_token: SealedCredential,
    },
    Zapier {
        hook_url: String,
        signing_secret: Option<SealedCredential>,
    },
    Make {
        hook_url: String,
        signing_secret: Option<SealedCredential>,
    },
    Sheets {
        spreadsheet_id: String,
        sheet: String,
        service_account_json: SealedCredential,
    },
    Notion {
        database_id: String,
        api_key: SealedCredential,
    },
    Airtable {
        base_id: String,
        table: String,
        api_key: SealedCredential,
    },
    Zoho {
        api_domain: String,
        access_token: SealedCredential,
    },
}

/// The dispatcher uses this stable id to log + retry per-integration
/// independently. Built from `(form_id, provider, list/hook id)` so the
/// outbox can de-dupe retries.
pub fn integration_id(form_id: Uuid, cfg: &IntegrationConfig) -> String {
    use IntegrationConfig::*;
    let (provider, scope) = match cfg {
        Mailchimp { list_id, .. } => ("mailchimp", list_id.clone()),
        ActiveCampaign { account_url, .. } => ("activecampaign", account_url.clone()),
        ConvertKit { form_id, .. } => ("convertkit", form_id.clone()),
        HubSpot { list_id, .. } => ("hubspot", list_id.clone().unwrap_or_default()),
        Salesforce { instance_url, .. } => ("salesforce", instance_url.clone()),
        Zapier { hook_url, .. } => ("zapier", hook_url.clone()),
        Make { hook_url, .. } => ("make", hook_url.clone()),
        Sheets { spreadsheet_id, .. } => ("sheets", spreadsheet_id.clone()),
        Notion { database_id, .. } => ("notion", database_id.clone()),
        Airtable { base_id, table, .. } => ("airtable", format!("{base_id}/{table}")),
        Zoho { api_domain, .. } => ("zoho", api_domain.clone()),
    };
    format!("{form_id}:{provider}:{scope}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic test-only cipher. Constructed from a fixed 32-byte
    /// pattern so every test sees the same key without any shared mutable
    /// state. Crucially: **this path never touches `APP_DATA_KEY` or any
    /// other environment variable**, so tests are free to run in parallel
    /// alongside the rest of the suite (and alongside any runtime machinery
    /// that happens to be reading env vars concurrently).
    fn test_cipher() -> Cipher {
        Cipher::from_key_bytes([42u8; 32])
    }

    #[test]
    fn round_trip_encrypt_decrypt() {
        let cipher = test_cipher();
        let s = SealedCredential::seal_with(&cipher, "super-secret-api-key").unwrap();
        let pt = s.unseal_with(&cipher).unwrap();
        assert_eq!(pt, "super-secret-api-key");
    }

    #[test]
    fn decrypt_with_tampered_ciphertext_fails() {
        let cipher = test_cipher();
        let s = SealedCredential::seal_with(&cipher, "hello").unwrap();
        // Decode → flip the first ciphertext byte → re-encode. Guarantees
        // a real bit-flip rather than relying on a substring being present.
        let mut bytes = B64.decode(&s.ciphertext).unwrap();
        bytes[0] ^= 0x01;
        let tampered = SealedCredential {
            ciphertext: B64.encode(&bytes),
            nonce: s.nonce.clone(),
        };
        assert!(matches!(
            tampered.unseal_with(&cipher),
            Err(CryptoError::DecryptFailed)
        ));
    }

    #[test]
    fn integration_id_is_stable() {
        let cfg = IntegrationConfig::Zapier {
            hook_url: "https://hooks.zapier.com/abc".into(),
            signing_secret: None,
        };
        let id = integration_id(Uuid::nil(), &cfg);
        assert_eq!(
            id,
            "00000000-0000-0000-0000-000000000000:zapier:https://hooks.zapier.com/abc"
        );
    }

    /// Regression guard for the CI flake where two env-mutating tests
    /// racing on `APP_DATA_KEY` produced intermittent `KeyMissing`
    /// failures under cargo's default parallel test harness. The current
    /// design relies on zero shared mutable state, so 32 threads each
    /// performing a full round-trip must all succeed deterministically.
    #[test]
    fn parallel_round_trips_are_deterministic() {
        let cipher = std::sync::Arc::new(test_cipher());
        let threads: Vec<_> = (0..32u32)
            .map(|i| {
                let cipher = std::sync::Arc::clone(&cipher);
                std::thread::spawn(move || {
                    let plaintext = format!("payload-{i}");
                    let sealed = SealedCredential::seal_with(&cipher, &plaintext).unwrap();
                    let unsealed = sealed.unseal_with(&cipher).unwrap();
                    assert_eq!(unsealed, plaintext);
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
    }

    /// Two independently-constructed ciphers must agree byte-for-byte as
    /// long as they were built from the same key bytes. This pins the
    /// invariant that `Cipher::from_key_bytes` is deterministic and does
    /// not mix in any hidden randomness.
    #[test]
    fn cipher_from_same_bytes_is_interchangeable() {
        let a = Cipher::from_key_bytes([7u8; 32]);
        let b = Cipher::from_key_bytes([7u8; 32]);
        let sealed = SealedCredential::seal_with(&a, "hello").unwrap();
        let unsealed = sealed.unseal_with(&b).unwrap();
        assert_eq!(unsealed, "hello");
    }

    /// Cross-key decryption must fail with the authenticated-decryption
    /// error rather than silently returning garbage.
    #[test]
    fn cipher_rejects_ciphertext_from_different_key() {
        let producer = Cipher::from_key_bytes([1u8; 32]);
        let consumer = Cipher::from_key_bytes([2u8; 32]);
        let sealed = SealedCredential::seal_with(&producer, "confidential").unwrap();
        assert!(matches!(
            sealed.unseal_with(&consumer),
            Err(CryptoError::DecryptFailed)
        ));
    }

    /// Debug must not leak the key. We don't assert on internal bytes —
    /// just that the redaction marker is present so accidental
    /// `dbg!(cipher)` lines during incident response do not page the key
    /// to logs.
    #[test]
    fn cipher_debug_redacts_key_material() {
        let dbg = format!("{:?}", test_cipher());
        assert!(dbg.contains("[redacted]"), "debug output = {dbg}");
    }

    /// `Cipher::from_base64` must reject inputs that decode to the wrong
    /// number of bytes — we rely on the type system (32-byte array) to
    /// enforce the invariant downstream, so this is a boundary test.
    #[test]
    fn cipher_from_base64_rejects_short_and_long_keys() {
        let short = B64.encode([0u8; 16]);
        let long = B64.encode([0u8; 64]);
        let not_base64 = "not valid base64!!!";
        assert!(matches!(
            Cipher::from_base64(&short),
            Err(CryptoError::KeyInvalid)
        ));
        assert!(matches!(
            Cipher::from_base64(&long),
            Err(CryptoError::KeyInvalid)
        ));
        assert!(matches!(
            Cipher::from_base64(not_base64),
            Err(CryptoError::KeyInvalid)
        ));
    }
}
