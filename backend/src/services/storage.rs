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
