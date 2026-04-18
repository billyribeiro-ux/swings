//! FORM-07: per-form integration configuration with envelope-encrypted
//! credentials.
//!
//! Each form's `settings.integrations` JSONB column stores a list of
//! [`IntegrationConfig`] entries. Secret material (API keys, OAuth refresh
//! tokens, service-account JWT private keys) is sealed with AES-256-GCM
//! using `APP_DATA_KEY` (32 bytes, base64 in env) and persisted as the
//! base64 ciphertext + base64 nonce. Plaintext only ever exists in memory
//! between [`decrypt`] and the integration's HTTP call.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
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

fn key_from_env() -> Result<Key<Aes256Gcm>, CryptoError> {
    let raw = std::env::var("APP_DATA_KEY").map_err(|_| CryptoError::KeyMissing)?;
    let bytes = B64.decode(raw).map_err(|_| CryptoError::KeyInvalid)?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::KeyInvalid)?;
    Ok(Key::<Aes256Gcm>::from(arr))
}

/// Encrypt a credential string with a freshly-generated 12-byte nonce.
/// Returns `(ciphertext_b64, nonce_b64)`. Both fields are persisted in
/// the form's settings JSON; rotation requires re-encrypting under the
/// new key (out of scope here — runbook'd separately).
pub fn encrypt(plaintext: &str) -> Result<(String, String), CryptoError> {
    let key = key_from_env()?;
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::EncryptFailed)?;
    Ok((B64.encode(ct), B64.encode(nonce)))
}

pub fn decrypt(ciphertext_b64: &str, nonce_b64: &str) -> Result<String, CryptoError> {
    let key = key_from_env()?;
    let cipher = Aes256Gcm::new(&key);
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

/// Encrypted-at-rest credential bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedCredential {
    pub ciphertext: String,
    pub nonce: String,
}

impl SealedCredential {
    pub fn seal(plaintext: &str) -> Result<Self, CryptoError> {
        let (ct, n) = encrypt(plaintext)?;
        Ok(Self {
            ciphertext: ct,
            nonce: n,
        })
    }

    pub fn unseal(&self) -> Result<String, CryptoError> {
        decrypt(&self.ciphertext, &self.nonce)
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

    fn with_test_key<F: FnOnce()>(f: F) {
        let key = [42u8; 32];
        let prev = std::env::var("APP_DATA_KEY").ok();
        // SAFETY: serial test, single-thread mode.
        std::env::set_var("APP_DATA_KEY", B64.encode(key));
        f();
        match prev {
            Some(v) => std::env::set_var("APP_DATA_KEY", v),
            None => std::env::remove_var("APP_DATA_KEY"),
        }
    }

    #[test]
    fn round_trip_encrypt_decrypt() {
        with_test_key(|| {
            let s = SealedCredential::seal("super-secret-api-key").unwrap();
            let pt = s.unseal().unwrap();
            assert_eq!(pt, "super-secret-api-key");
        });
    }

    #[test]
    fn decrypt_with_tampered_ciphertext_fails() {
        with_test_key(|| {
            let s = SealedCredential::seal("hello").unwrap();
            // Decode → flip the first ciphertext byte → re-encode. Guarantees
            // a real bit-flip rather than relying on a substring being present.
            let mut bytes = B64.decode(&s.ciphertext).unwrap();
            bytes[0] ^= 0x01;
            let tampered = SealedCredential {
                ciphertext: B64.encode(&bytes),
                nonce: s.nonce.clone(),
            };
            assert!(matches!(tampered.unseal(), Err(CryptoError::DecryptFailed)));
        });
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
}
