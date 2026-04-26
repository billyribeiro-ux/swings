use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use bytes::Bytes;
use std::env;

/// Cloudflare R2 (S3-compatible) storage.
#[derive(Clone)]
pub struct R2Storage {
    client: Client,
    bucket: String,
    public_base: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    #[error("R2 upload failed: {0}")]
    Upload(String),
    #[error("R2 delete failed: {0}")]
    Delete(String),
    #[error("R2 configuration error: {0}")]
    Config(String),
}

impl R2Storage {
    /// Load R2 from environment. All of `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`,
    /// `R2_BUCKET_NAME`, `R2_PUBLIC_URL` must be set and non-empty.
    pub fn from_env() -> Result<Self, StorageError> {
        let account_id = env::var("R2_ACCOUNT_ID")
            .map_err(|_| StorageError::Config("R2_ACCOUNT_ID not set".into()))?;
        if account_id.trim().is_empty() {
            return Err(StorageError::Config("R2_ACCOUNT_ID is empty".into()));
        }
        let access_key = env::var("R2_ACCESS_KEY_ID")
            .map_err(|_| StorageError::Config("R2_ACCESS_KEY_ID not set".into()))?;
        let secret_key = env::var("R2_SECRET_ACCESS_KEY")
            .map_err(|_| StorageError::Config("R2_SECRET_ACCESS_KEY not set".into()))?;
        let bucket = env::var("R2_BUCKET_NAME")
            .map_err(|_| StorageError::Config("R2_BUCKET_NAME not set".into()))?;
        let public_url = env::var("R2_PUBLIC_URL")
            .map_err(|_| StorageError::Config("R2_PUBLIC_URL not set".into()))?;

        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id.trim());
        let credentials = aws_sdk_s3::config::Credentials::new(
            access_key.trim(),
            secret_key.trim(),
            None,
            None,
            "r2",
        );

        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .credentials_provider(credentials)
            .region(aws_sdk_s3::config::Region::new("auto"))
            .force_path_style(true)
            .build();

        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: bucket.trim().to_string(),
            public_base: public_url.trim_end_matches('/').to_string(),
        })
    }

    /// Upload bytes; returns public URL.
    pub async fn upload(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<String, StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .content_type(content_type)
            .cache_control("public, max-age=31536000, immutable")
            .send()
            .await
            .map_err(|e| StorageError::Upload(e.to_string()))?;

        Ok(self.public_url_for_key(key))
    }

    /// EC-07: produce a short-lived (TTL-bounded) presigned GET URL for a
    /// downloadable asset. The URL is valid for the supplied duration and
    /// MUST NOT be reused — the consume_download repo helper decrements
    /// the download quota before this call so a stolen link cannot be
    /// replayed past its TTL or quota.
    pub async fn presign_get(
        &self,
        key: &str,
        ttl: std::time::Duration,
    ) -> Result<String, StorageError> {
        let presigning = aws_sdk_s3::presigning::PresigningConfig::expires_in(ttl)
            .map_err(|e| StorageError::Config(format!("presigning: {e}")))?;
        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning)
            .await
            .map_err(|e| StorageError::Upload(e.to_string()))?;
        Ok(req.uri().to_string())
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Delete(e.to_string()))?;
        Ok(())
    }

    /// Build an `R2Storage` against an arbitrary S3-compatible
    /// endpoint, used by tests + dev fixtures that point at MinIO,
    /// LocalStack, or a self-hosted Ceph/Garage node.
    ///
    /// Production code should keep using [`Self::from_env`] which
    /// hardcodes the `*.r2.cloudflarestorage.com` endpoint to match
    /// the credentials Cloudflare issues. This constructor only
    /// exists so we can run integration tests against the same
    /// upload + presign + delete code paths the production worker
    /// uses, without a real Cloudflare account.
    ///
    /// `region` is mostly ignored by S3-compatible servers but must
    /// be a non-empty string for the SDK; pass `"us-east-1"` if you
    /// have no preference. `force_path_style` is always on so we
    /// work with single-host emulators (MinIO etc.) that don't
    /// resolve virtual-hosted bucket subdomains.
    pub fn for_endpoint(
        endpoint: impl Into<String>,
        region: impl Into<String>,
        bucket: impl Into<String>,
        public_base: impl Into<String>,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        let credentials = aws_sdk_s3::config::Credentials::new(
            access_key.into(),
            secret_key.into(),
            None,
            None,
            "for-endpoint",
        );
        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .endpoint_url(endpoint.into())
            .credentials_provider(credentials)
            .region(aws_sdk_s3::config::Region::new(region.into()))
            .force_path_style(true)
            .build();
        Self {
            client: Client::from_conf(s3_config),
            bucket: bucket.into(),
            public_base: public_base.into().trim_end_matches('/').to_string(),
        }
    }

    /// Create the configured bucket if it does not already exist.
    /// Used by tests to bootstrap a fresh emulator; in production
    /// the bucket is created out-of-band by IaC.
    pub async fn ensure_bucket(&self) -> Result<(), StorageError> {
        // `head_bucket` is the cheapest probe; if it 404s we create.
        let head = self.client.head_bucket().bucket(&self.bucket).send().await;
        if head.is_ok() {
            return Ok(());
        }
        self.client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| StorageError::Config(format!("create_bucket: {e}")))
    }

    /// Probe-only existence check on a key. `Ok(true)` means the
    /// object exists, `Ok(false)` means it does not, and `Err` is
    /// reserved for transport / config failures. Used by the TTL
    /// sweep tests to assert the R2 object is actually gone after
    /// the worker scrubs the row.
    pub async fn object_exists(&self, key: &str) -> Result<bool, StorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            // S3 wraps the 404 in a service error — match on the
            // textual marker rather than coupling to the SDK's
            // generated error variants (which churn between
            // versions of `aws-sdk-s3`).
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("NotFound") || msg.contains("NoSuchKey") {
                    Ok(false)
                } else {
                    Err(StorageError::Upload(msg))
                }
            }
        }
    }

    /// `media/{year}/{month}/{uuid8}-{sanitized_name}`
    pub fn generate_key(original_filename: &str) -> String {
        let now = chrono::Utc::now();
        let sanitized = sanitize_filename::sanitize(original_filename);
        let unique: String = uuid::Uuid::new_v4().to_string().chars().take(8).collect();
        format!(
            "media/{}/{:02}/{}-{}",
            now.format("%Y"),
            now.format("%m"),
            unique,
            sanitized
        )
    }

    pub fn public_url_for_key(&self, key: &str) -> String {
        format!("{}/{}", self.public_base, key)
    }
}

#[cfg(test)]
mod tests {
    //! Phase 8.11 — pure-function coverage for the storage helpers.
    //!
    //! Network-touching paths (`upload`, `presign_get`, `delete_object`,
    //! `ensure_bucket`, `object_exists`) require a live S3-compatible
    //! endpoint and are exercised by separate integration tests against
    //! MinIO; here we only assert the deterministic helpers that should
    //! never make a network call.

    use super::*;

    #[test]
    fn generate_key_uses_media_year_month_prefix() {
        let key = R2Storage::generate_key("photo.jpg");
        // `media/{YYYY}/{MM}/{8-hex}-photo.jpg`
        let segments: Vec<&str> = key.splitn(4, '/').collect();
        assert_eq!(segments.len(), 4, "key shape: {key}");
        assert_eq!(segments[0], "media");
        assert_eq!(segments[1].len(), 4); // 4-digit year
        assert!(segments[1].chars().all(|c| c.is_ascii_digit()));
        assert_eq!(segments[2].len(), 2); // zero-padded month
        assert!(segments[2].chars().all(|c| c.is_ascii_digit()));
        // Filename half: 8-char id + dash + sanitized name.
        let tail = segments[3];
        assert!(tail.contains("-photo.jpg"), "tail without sanitized name: {tail}");
        let id_part = tail.split('-').next().expect("split before dash");
        assert_eq!(id_part.len(), 8);
    }

    #[test]
    fn generate_key_is_unique_across_calls() {
        let a = R2Storage::generate_key("file.txt");
        let b = R2Storage::generate_key("file.txt");
        assert_ne!(a, b, "uuid prefix must differ; got {a} == {b}");
    }

    #[test]
    fn generate_key_sanitizes_dangerous_filenames() {
        // `sanitize_filename` strips path-traversal segments + null bytes.
        let key = R2Storage::generate_key("../../etc/passwd");
        // The key remains anchored on `media/{Y}/{M}/`; nothing should
        // escape that prefix.
        assert!(key.starts_with("media/"));
        assert!(!key.contains("../"), "path traversal not stripped: {key}");
    }

    #[test]
    fn public_url_for_key_joins_with_single_slash() {
        let s = R2Storage::for_endpoint(
            "https://emulator.example",
            "us-east-1",
            "bucket",
            "https://cdn.example.test", // no trailing slash
            "ak",
            "sk",
        );
        assert_eq!(
            s.public_url_for_key("media/2026/01/abcdef12-photo.jpg"),
            "https://cdn.example.test/media/2026/01/abcdef12-photo.jpg"
        );
    }

    #[test]
    fn public_url_for_key_strips_trailing_slash_from_base() {
        // `for_endpoint` trims a trailing slash from `public_base` so the
        // join never produces `//`.
        let s = R2Storage::for_endpoint(
            "https://emulator.example",
            "us-east-1",
            "bucket",
            "https://cdn.example.test/", // trailing slash
            "ak",
            "sk",
        );
        assert_eq!(s.public_url_for_key("foo.png"), "https://cdn.example.test/foo.png");
    }

    #[test]
    fn media_backend_is_r2_reflects_variant() {
        let local = MediaBackend::Local {
            upload_dir: "/tmp/u".into(),
        };
        assert!(!local.is_r2());
        assert_eq!(local.upload_dir(), Some("/tmp/u"));
    }
}

/// Media persistence: local disk (dev) or Cloudflare R2 (production).
#[derive(Clone)]
pub enum MediaBackend {
    Local { upload_dir: String },
    R2(R2Storage),
}

impl MediaBackend {
    /// Prefer R2 when all variables are set; otherwise local `upload_dir`.
    pub fn resolve(upload_dir: String) -> Self {
        match R2Storage::from_env() {
            Ok(r2) => {
                tracing::info!("Media backend: R2 (bucket configured)");
                MediaBackend::R2(r2)
            }
            Err(e) => {
                tracing::warn!(
                    "R2 not configured ({}); using local uploads at {}",
                    e,
                    upload_dir
                );
                MediaBackend::Local { upload_dir }
            }
        }
    }

    pub fn is_r2(&self) -> bool {
        matches!(self, MediaBackend::R2(_))
    }

    pub fn upload_dir(&self) -> Option<&str> {
        match self {
            MediaBackend::Local { upload_dir } => Some(upload_dir.as_str()),
            MediaBackend::R2(_) => None,
        }
    }

    /// Delete a previously uploaded object. R2 path delegates to
    /// [`R2Storage::delete_object`]; local path removes the file at
    /// `{upload_dir}/{key}`. Both branches treat a missing object as
    /// success — TTL sweepers commonly race with manual cleanup, and
    /// surfacing `NotFound` as an error would just spam the logs.
    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        match self {
            MediaBackend::R2(r2) => match r2.delete_object(key).await {
                Ok(()) => Ok(()),
                // S3 surfaces `NoSuchKey` as a 404; treat it as a no-op.
                Err(StorageError::Delete(msg)) if msg.contains("NoSuchKey") => Ok(()),
                Err(e) => Err(e),
            },
            MediaBackend::Local { upload_dir } => {
                let path = std::path::Path::new(upload_dir).join(key);
                match tokio::fs::remove_file(&path).await {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(StorageError::Delete(format!(
                        "remove {}: {e}",
                        path.display()
                    ))),
                }
            }
        }
    }
}
