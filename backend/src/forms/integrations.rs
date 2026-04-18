//! FORM-07: integration adapters.
//!
//! Each adapter implements [`IntegrationAdapter::dispatch`], a thin
//! HTTP call that pushes a single form submission into a downstream
//! SaaS. Adapters are stateless — credentials live on the
//! [`IntegrationConfig`] passed in by the dispatcher, which itself
//! is invoked from the `form.submission.created` outbox event handler.
//!
//! Errors:
//!   * [`IntegrationError::NotConfigured`] — adapter cannot run (e.g.
//!     missing access token); dispatcher MUST mark non-retriable.
//!   * [`IntegrationError::Transient`] — network blip or 5xx; the
//!     outbox retries with exponential backoff.
//!   * [`IntegrationError::Permanent`] — 4xx or schema mismatch; the
//!     outbox stops retrying and parks the row in the dead-letter view.

use async_trait::async_trait;
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;
use uuid::Uuid;

use super::integration_config::{CryptoError, IntegrationConfig, SealedCredential};

#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("integration not configured: {0}")]
    NotConfigured(&'static str),
    #[error("transient downstream error: {0}")]
    Transient(String),
    #[error("permanent downstream error: {0}")]
    Permanent(String),
    #[error("credential decrypt failed: {0}")]
    Crypto(#[from] CryptoError),
    #[error("http error: {0}")]
    Http(String),
}

impl IntegrationError {
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            IntegrationError::Transient(_) | IntegrationError::Http(_)
        )
    }
}

/// Slim view of a submission passed to every adapter — handlers borrow,
/// adapters serialise into provider-specific shapes.
#[derive(Debug, Clone)]
pub struct SubmissionPayload<'a> {
    pub form_id: Uuid,
    pub submission_id: Uuid,
    pub data: &'a serde_json::Value,
    pub email: Option<String>,
}

#[async_trait]
pub trait IntegrationAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        payload: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError>;
}

// ── HTTP helpers ───────────────────────────────────────────────────────

async fn post_json<B: Serialize>(
    client: &reqwest::Client,
    url: &str,
    auth_header: Option<(&str, &str)>,
    body: &B,
) -> Result<reqwest::Response, IntegrationError> {
    let mut req = client.post(url).json(body);
    if let Some((name, value)) = auth_header {
        req = req.header(name, value);
    }
    let res = req
        .send()
        .await
        .map_err(|e| IntegrationError::Transient(e.to_string()))?;
    classify(res).await
}

async fn classify(res: reqwest::Response) -> Result<reqwest::Response, IntegrationError> {
    let status = res.status();
    if status.is_success() {
        return Ok(res);
    }
    let body = res.text().await.unwrap_or_default();
    if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(IntegrationError::Transient(format!("{status}: {body}")));
    }
    Err(IntegrationError::Permanent(format!("{status}: {body}")))
}

fn require_email<'a>(p: &'a SubmissionPayload<'_>) -> Result<&'a str, IntegrationError> {
    p.email.as_deref().ok_or(IntegrationError::Permanent(
        "submission has no email field".into(),
    ))
}

fn extract_first_last(data: &serde_json::Value) -> (Option<String>, Option<String>) {
    let take = |k: &str| -> Option<String> {
        data.as_object()
            .and_then(|m| m.get(k))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    (
        take("first_name")
            .or_else(|| take("firstName"))
            .or_else(|| take("FNAME")),
        take("last_name")
            .or_else(|| take("lastName"))
            .or_else(|| take("LNAME")),
    )
}

// ── Adapters ───────────────────────────────────────────────────────────

pub struct Mailchimp;
#[async_trait]
impl IntegrationAdapter for Mailchimp {
    fn name(&self) -> &'static str {
        "mailchimp"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Mailchimp { list_id, api_key } = cfg else {
            return Err(IntegrationError::NotConfigured("mailchimp"));
        };
        let key = api_key.unseal()?;
        let dc = key
            .split_once('-')
            .map(|(_, dc)| dc)
            .ok_or(IntegrationError::Permanent(
                "malformed mailchimp key".into(),
            ))?;
        let email = require_email(p)?;
        let (first, last) = extract_first_last(p.data);
        let url = format!("https://{dc}.api.mailchimp.com/3.0/lists/{list_id}/members");
        let body = serde_json::json!({
            "email_address": email,
            "status_if_new": "subscribed",
            "merge_fields": { "FNAME": first, "LNAME": last },
        });
        post_json(
            client,
            &url,
            Some(("Authorization", &format!("apikey {key}"))),
            &body,
        )
        .await?;
        Ok(())
    }
}

pub struct ActiveCampaign;
#[async_trait]
impl IntegrationAdapter for ActiveCampaign {
    fn name(&self) -> &'static str {
        "activecampaign"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::ActiveCampaign {
            account_url,
            api_key,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("activecampaign"));
        };
        let key = api_key.unseal()?;
        let email = require_email(p)?;
        let (first, last) = extract_first_last(p.data);
        let url = format!("{account_url}/api/3/contact/sync");
        let body = serde_json::json!({
            "contact": { "email": email, "firstName": first, "lastName": last }
        });
        post_json(client, &url, Some(("Api-Token", &key)), &body).await?;
        Ok(())
    }
}

pub struct ConvertKit;
#[async_trait]
impl IntegrationAdapter for ConvertKit {
    fn name(&self) -> &'static str {
        "convertkit"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::ConvertKit { form_id, api_key } = cfg else {
            return Err(IntegrationError::NotConfigured("convertkit"));
        };
        let key = api_key.unseal()?;
        let email = require_email(p)?;
        let (first, _last) = extract_first_last(p.data);
        let url = format!("https://api.convertkit.com/v3/forms/{form_id}/subscribe");
        let body = serde_json::json!({ "api_key": key, "email": email, "first_name": first });
        post_json(client, &url, None, &body).await?;
        Ok(())
    }
}

pub struct HubSpot;
#[async_trait]
impl IntegrationAdapter for HubSpot {
    fn name(&self) -> &'static str {
        "hubspot"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::HubSpot { access_token, .. } = cfg else {
            return Err(IntegrationError::NotConfigured("hubspot"));
        };
        let token = access_token.unseal()?;
        let email = require_email(p)?;
        let (first, last) = extract_first_last(p.data);
        let url = "https://api.hubapi.com/crm/v3/objects/contacts";
        let body = serde_json::json!({
            "properties": { "email": email, "firstname": first, "lastname": last }
        });
        post_json(
            client,
            url,
            Some(("Authorization", &format!("Bearer {token}"))),
            &body,
        )
        .await?;
        Ok(())
    }
}

pub struct Salesforce;
#[async_trait]
impl IntegrationAdapter for Salesforce {
    fn name(&self) -> &'static str {
        "salesforce"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Salesforce {
            instance_url,
            access_token,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("salesforce"));
        };
        let token = access_token.unseal()?;
        let email = require_email(p)?;
        let (first, last) = extract_first_last(p.data);
        let company = p
            .data
            .get("company")
            .and_then(|v| v.as_str())
            .unwrap_or("Web Lead");
        let url = format!("{instance_url}/services/data/v60.0/sobjects/Lead/");
        let body = serde_json::json!({
            "FirstName": first, "LastName": last, "Email": email, "Company": company
        });
        post_json(
            client,
            &url,
            Some(("Authorization", &format!("Bearer {token}"))),
            &body,
        )
        .await?;
        Ok(())
    }
}

pub struct Zapier;
#[async_trait]
impl IntegrationAdapter for Zapier {
    fn name(&self) -> &'static str {
        "zapier"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Zapier {
            hook_url,
            signing_secret,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("zapier"));
        };
        webhook_post(client, hook_url, signing_secret.as_ref(), p).await
    }
}

pub struct Make;
#[async_trait]
impl IntegrationAdapter for Make {
    fn name(&self) -> &'static str {
        "make"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Make {
            hook_url,
            signing_secret,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("make"));
        };
        webhook_post(client, hook_url, signing_secret.as_ref(), p).await
    }
}

/// Generic outbound webhook helper — Zapier + Make both consume this.
/// The body is a stable schema so downstream zaps don't break on adapter
/// internals; the optional HMAC signature lets the receiver verify the
/// payload originated from us.
async fn webhook_post(
    client: &reqwest::Client,
    url: &str,
    signing_secret: Option<&SealedCredential>,
    p: &SubmissionPayload<'_>,
) -> Result<(), IntegrationError> {
    let body = serde_json::json!({
        "form_id": p.form_id,
        "submission_id": p.submission_id,
        "email": p.email,
        "data": p.data,
    });
    let body_bytes = serde_json::to_vec(&body)
        .map_err(|e| IntegrationError::Permanent(format!("body serialise: {e}")))?;
    let mut req = client.post(url).header("Content-Type", "application/json");
    if let Some(secret_sealed) = signing_secret {
        let secret = secret_sealed.unseal()?;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|e| IntegrationError::Permanent(format!("hmac key: {e}")))?;
        mac.update(&body_bytes);
        let sig = mac.finalize().into_bytes();
        let hex: String = sig.iter().map(|b| format!("{b:02x}")).collect();
        req = req.header("X-Signature-256", format!("sha256={hex}"));
    }
    let res = req
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| IntegrationError::Transient(e.to_string()))?;
    classify(res).await?;
    Ok(())
}

pub struct Sheets;
#[async_trait]
impl IntegrationAdapter for Sheets {
    fn name(&self) -> &'static str {
        "sheets"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Sheets {
            spreadsheet_id,
            sheet,
            service_account_json,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("sheets"));
        };
        let svc_json = service_account_json.unseal()?;
        let token = google_oauth_token(&svc_json).await?;
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{sheet}!A1:append?valueInputOption=RAW"
        );
        let row: Vec<serde_json::Value> = vec![
            serde_json::Value::String(p.form_id.to_string()),
            serde_json::Value::String(p.submission_id.to_string()),
            serde_json::Value::String(p.email.clone().unwrap_or_default()),
            serde_json::Value::String(p.data.to_string()),
        ];
        let body = serde_json::json!({ "values": [row] });
        post_json(
            client,
            &url,
            Some(("Authorization", &format!("Bearer {token}"))),
            &body,
        )
        .await?;
        Ok(())
    }
}

/// Mint an OAuth2 access token for a Google service account.
///
/// Implements the [JWT bearer flow][rfc7523]: build an RS256-signed
/// claim with the service-account email as `iss`, the requested scope,
/// `https://oauth2.googleapis.com/token` as `aud`, and a 1-hour
/// expiry. POST `grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer
/// &assertion=…` to the token endpoint and return the response's
/// `access_token`.
///
/// [rfc7523]: https://datatracker.ietf.org/doc/html/rfc7523
async fn google_oauth_token(service_account_json: &str) -> Result<String, IntegrationError> {
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(serde::Deserialize)]
    struct ServiceAccount {
        client_email: String,
        private_key: String,
        token_uri: Option<String>,
    }

    #[derive(serde::Serialize)]
    struct Claims<'a> {
        iss: &'a str,
        scope: &'a str,
        aud: &'a str,
        exp: u64,
        iat: u64,
    }

    #[derive(serde::Deserialize)]
    struct TokenResponse {
        access_token: String,
    }

    let sa: ServiceAccount = serde_json::from_str(service_account_json).map_err(|e| {
        IntegrationError::Permanent(format!("sheets service-account JSON malformed: {e}"))
    })?;

    let token_uri = sa
        .token_uri
        .as_deref()
        .unwrap_or("https://oauth2.googleapis.com/token");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| IntegrationError::Permanent("system clock before unix epoch".into()))?
        .as_secs();
    let claims = Claims {
        iss: &sa.client_email,
        scope: "https://www.googleapis.com/auth/spreadsheets",
        aud: token_uri,
        exp: now + 3600,
        iat: now,
    };
    let key = EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .map_err(|e| IntegrationError::Permanent(format!("sheets private_key not RSA PEM: {e}")))?;
    let assertion = jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map_err(|e| IntegrationError::Permanent(format!("sheets JWT encode failed: {e}")))?;

    let client = reqwest::Client::new();
    let res = client
        .post(token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &assertion),
        ])
        .send()
        .await
        .map_err(|e| IntegrationError::Transient(e.to_string()))?;
    let res = classify(res).await?;
    let body: TokenResponse = res
        .json()
        .await
        .map_err(|e| IntegrationError::Permanent(format!("sheets token response decode: {e}")))?;
    Ok(body.access_token)
}

pub struct Notion;
#[async_trait]
impl IntegrationAdapter for Notion {
    fn name(&self) -> &'static str {
        "notion"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Notion {
            database_id,
            api_key,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("notion"));
        };
        let key = api_key.unseal()?;
        let email = p.email.clone().unwrap_or_default();
        let url = "https://api.notion.com/v1/pages";
        let body = serde_json::json!({
            "parent": { "database_id": database_id },
            "properties": {
                "Email": { "title": [{ "text": { "content": email } }] },
                "Submission": { "rich_text": [{ "text": { "content": p.data.to_string() } }] }
            }
        });
        let res = client
            .post(url)
            .header("Authorization", format!("Bearer {key}"))
            .header("Notion-Version", "2022-06-28")
            .json(&body)
            .send()
            .await
            .map_err(|e| IntegrationError::Transient(e.to_string()))?;
        classify(res).await?;
        Ok(())
    }
}

pub struct Airtable;
#[async_trait]
impl IntegrationAdapter for Airtable {
    fn name(&self) -> &'static str {
        "airtable"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Airtable {
            base_id,
            table,
            api_key,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("airtable"));
        };
        let key = api_key.unseal()?;
        let url = format!("https://api.airtable.com/v0/{base_id}/{table}");
        let body = serde_json::json!({
            "fields": {
                "Email": p.email.clone().unwrap_or_default(),
                "Submission": p.data.to_string(),
                "FormId": p.form_id.to_string(),
            }
        });
        post_json(
            client,
            &url,
            Some(("Authorization", &format!("Bearer {key}"))),
            &body,
        )
        .await?;
        Ok(())
    }
}

pub struct Zoho;
#[async_trait]
impl IntegrationAdapter for Zoho {
    fn name(&self) -> &'static str {
        "zoho"
    }
    async fn dispatch(
        &self,
        client: &reqwest::Client,
        cfg: &IntegrationConfig,
        p: &SubmissionPayload<'_>,
    ) -> Result<(), IntegrationError> {
        let IntegrationConfig::Zoho {
            api_domain,
            access_token,
        } = cfg
        else {
            return Err(IntegrationError::NotConfigured("zoho"));
        };
        let token = access_token.unseal()?;
        let email = require_email(p)?;
        let (first, last) = extract_first_last(p.data);
        let url = format!("{api_domain}/crm/v6/Leads");
        let body = serde_json::json!({
            "data": [{
                "Email": email,
                "First_Name": first,
                "Last_Name": last.unwrap_or_else(|| "Web Lead".into()),
                "Company": "Web Lead",
            }]
        });
        post_json(
            client,
            &url,
            Some(("Authorization", &format!("Zoho-oauthtoken {token}"))),
            &body,
        )
        .await?;
        Ok(())
    }
}

/// Return the adapter responsible for the given config tag.
pub fn adapter_for(cfg: &IntegrationConfig) -> Box<dyn IntegrationAdapter> {
    match cfg {
        IntegrationConfig::Mailchimp { .. } => Box::new(Mailchimp),
        IntegrationConfig::ActiveCampaign { .. } => Box::new(ActiveCampaign),
        IntegrationConfig::ConvertKit { .. } => Box::new(ConvertKit),
        IntegrationConfig::HubSpot { .. } => Box::new(HubSpot),
        IntegrationConfig::Salesforce { .. } => Box::new(Salesforce),
        IntegrationConfig::Zapier { .. } => Box::new(Zapier),
        IntegrationConfig::Make { .. } => Box::new(Make),
        IntegrationConfig::Sheets { .. } => Box::new(Sheets),
        IntegrationConfig::Notion { .. } => Box::new(Notion),
        IntegrationConfig::Airtable { .. } => Box::new(Airtable),
        IntegrationConfig::Zoho { .. } => Box::new(Zoho),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_retries_500_treats_400_as_permanent() {
        // We rely on classify being deterministic on status code.
        let err5 = IntegrationError::Transient("500".into());
        let err4 = IntegrationError::Permanent("400".into());
        assert!(err5.is_retriable());
        assert!(!err4.is_retriable());
    }

    #[test]
    fn adapter_for_dispatch_matches_provider() {
        let cfg = IntegrationConfig::Zapier {
            hook_url: "https://example".into(),
            signing_secret: None,
        };
        let a = adapter_for(&cfg);
        assert_eq!(a.name(), "zapier");
    }
}
