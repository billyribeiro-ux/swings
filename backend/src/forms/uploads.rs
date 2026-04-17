//! FORM-05: file-upload pipeline.
//!
//! The public endpoint `POST /api/forms/{slug}/upload` streams a multipart
//! body into memory in bounded chunks, sniffs the MIME of the first 512
//! bytes with `infer`, enforces the field's `FileRules` (count / MIME /
//! size), commits the bytes to a [`StorageProvider`], then records the
//! descriptor in `form_uploads` via [`crate::forms::repo::insert_upload`].
//!
//! Chunked uploads: RFC 7233 `Content-Range: bytes start-end/total` is
//! supported — successive calls append onto the same in-memory buffer
//! keyed by a client-supplied `X-Upload-Session` UUID. The buffer is
//! evicted either when the total has been received or when the session
//! ages past [`UPLOAD_SESSION_TTL`] (whichever comes first).
//!
//! Storage: the trait is the seam that lets tests use
//! [`InMemoryStorage`] without any AWS SDK, while production wires
//! through to S3/R2 via `aws-sdk-s3`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::schema::FileRules;

/// Cap on buffered upload bytes per session. Matches the hard ceiling set
/// on `FileRules.max_file_size` across shipped forms; smaller fields get
/// a smaller cap via the schema-level check in [`sniff_and_enforce`].
pub const MAX_UPLOAD_BYTES: u64 = 50 * 1024 * 1024;

/// How long a chunked upload session may remain idle before the buffer is
/// evicted. Chosen to be larger than the longest reasonable transfer over a
/// throttled 3G link (50 MB at 128 kbps ≈ 53 min) with headroom.
pub const UPLOAD_SESSION_TTL: Duration = Duration::from_secs(60 * 60);

// ── Content-Range parsing ──────────────────────────────────────────────

/// Parsed `Content-Range: bytes start-end/total` header. The server rejects
/// ranges whose `total` exceeds [`MAX_UPLOAD_BYTES`] or whose `end` is past
/// `total - 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub start: u64,
    pub end: u64,
    pub total: u64,
}

impl ContentRange {
    /// Parse the RFC 7233 shape. Returns `None` on any formatting error so
    /// the handler can map to 400.
    pub fn parse(header: &str) -> Option<Self> {
        let rest = header.strip_prefix("bytes ")?.trim();
        let (range, total) = rest.split_once('/')?;
        let (start, end) = range.split_once('-')?;
        let start: u64 = start.parse().ok()?;
        let end: u64 = end.parse().ok()?;
        let total: u64 = total.parse().ok()?;
        if end < start || end >= total || total > MAX_UPLOAD_BYTES {
            return None;
        }
        Some(ContentRange { start, end, total })
    }

    pub fn len(&self) -> u64 {
        self.end - self.start + 1
    }

    pub fn is_last(&self) -> bool {
        self.end + 1 == self.total
    }
}

// ── MIME sniff + rule enforcement ──────────────────────────────────────

/// Error shape for the upload pipeline. Kept private to the module so
/// handlers convert via [`From`] when propagating via `?`.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("upload exceeds allowed size for this field")]
    TooLarge,
    #[error("server-detected MIME `{0}` is not allowed for this field")]
    MimeRejected(String),
    #[error("invalid Content-Range header")]
    BadRange,
    #[error("upload session `{0}` has expired; restart the upload")]
    SessionExpired(Uuid),
}

/// Sniff the MIME of the first 512 bytes and enforce the field's rules.
/// Returns the detected MIME on success.
pub fn sniff_and_enforce(bytes: &[u8], rules: &FileRules) -> Result<String, UploadError> {
    if let Some(max) = rules.max_file_size {
        if (bytes.len() as u64) > max {
            return Err(UploadError::TooLarge);
        }
    }
    let detected = infer::get(bytes)
        .map(|t| t.mime_type().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    if !rules.allowed_mime_types.is_empty() && !rules.allowed_mime_types.iter().any(|m| m == &detected)
    {
        return Err(UploadError::MimeRejected(detected));
    }
    Ok(detected)
}

// ── Storage seam ───────────────────────────────────────────────────────

/// Backing store trait. Implementations must be idempotent on `put` for
/// the same `key`: the handler retries on transient network errors and
/// must not end up with two copies under different keys.
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn put(&self, key: &str, bytes: &[u8], content_type: &str) -> Result<(), UploadError>;
    async fn head(&self, key: &str) -> Result<Option<u64>, UploadError>;
}

/// Test-only store. Not exposed outside `#[cfg(test)]` paths in prod builds
/// but left compile-visible so integration tests can share it.
#[derive(Default, Clone)]
pub struct InMemoryStorage {
    inner: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

#[async_trait]
impl StorageProvider for InMemoryStorage {
    async fn put(&self, key: &str, bytes: &[u8], _content_type: &str) -> Result<(), UploadError> {
        let mut g = self.inner.lock().expect("in-memory storage mutex poisoned");
        g.insert(key.to_string(), bytes.to_vec());
        Ok(())
    }

    async fn head(&self, key: &str) -> Result<Option<u64>, UploadError> {
        let g = self.inner.lock().expect("in-memory storage mutex poisoned");
        Ok(g.get(key).map(|v| v.len() as u64))
    }
}

// ── Chunk assembler ────────────────────────────────────────────────────

/// Keyed live-chunk buffer. Handlers construct one [`ChunkedUploadStore`]
/// per AppState and reuse it across requests; it evicts idle sessions
/// lazily on every `append`.
#[derive(Default)]
pub struct ChunkedUploadStore {
    sessions: Mutex<HashMap<Uuid, Session>>,
}

struct Session {
    buf: Vec<u8>,
    total: u64,
    last_touch: Instant,
}

impl ChunkedUploadStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a chunk. Returns the assembled bytes when the final range has
    /// been written, `None` otherwise. Bad ranges bubble up as
    /// [`UploadError::BadRange`].
    pub fn append(
        &self,
        session: Uuid,
        range: ContentRange,
        chunk: &[u8],
    ) -> Result<Option<Vec<u8>>, UploadError> {
        if chunk.len() as u64 != range.len() {
            return Err(UploadError::BadRange);
        }
        let mut g = self.sessions.lock().expect("chunk store mutex poisoned");

        // Evict stale sessions opportunistically.
        g.retain(|_, s| s.last_touch.elapsed() < UPLOAD_SESSION_TTL);

        let entry = g.entry(session).or_insert_with(|| Session {
            buf: Vec::with_capacity(range.total as usize),
            total: range.total,
            last_touch: Instant::now(),
        });
        if entry.total != range.total {
            return Err(UploadError::BadRange);
        }
        if entry.buf.len() as u64 != range.start {
            // Non-contiguous append — client must restart the session.
            return Err(UploadError::BadRange);
        }
        entry.buf.extend_from_slice(chunk);
        entry.last_touch = Instant::now();

        if range.is_last() {
            let s = g.remove(&session).expect("session just touched");
            return Ok(Some(s.buf));
        }
        Ok(None)
    }
}

// ── Finalisation ───────────────────────────────────────────────────────

/// Outcome of a finalised upload — the descriptor the handler persists.
#[derive(Debug, Clone)]
pub struct StoredUpload {
    pub storage_key: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub sha256: [u8; 32],
}

/// Build the storage key in the plan's canonical layout:
///   `forms/{form_id}/{partial_or_submission_id}/{upload_id}-{safe_name}`
pub fn make_storage_key(
    form_id: Uuid,
    partial_or_submission_id: Uuid,
    upload_id: Uuid,
    filename: &str,
) -> String {
    let safe = sanitize_filename::sanitize(filename);
    format!("forms/{form_id}/{partial_or_submission_id}/{upload_id}-{safe}")
}

/// Hash + write the bytes to the storage provider. Returns the descriptor
/// used to build the `form_uploads` row.
pub async fn finalize_upload(
    storage: &dyn StorageProvider,
    storage_key: &str,
    bytes: &[u8],
    rules: &FileRules,
) -> Result<StoredUpload, UploadError> {
    let mime = sniff_and_enforce(bytes, rules)?;
    let size_bytes = i64::try_from(bytes.len()).map_err(|_| UploadError::TooLarge)?;
    let sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest: [u8; 32] = hasher.finalize().into();
        digest
    };
    storage.put(storage_key, bytes, &mime).await?;
    Ok(StoredUpload {
        storage_key: storage_key.to_string(),
        mime_type: mime,
        size_bytes,
        sha256,
    })
}

// ── Unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rules_png() -> FileRules {
        FileRules {
            min_files: None,
            max_files: None,
            allowed_mime_types: vec!["image/png".into()],
            max_file_size: Some(1024),
        }
    }

    // 8-byte PNG signature — enough for `infer` to detect.
    const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn content_range_parses_standard_shape() {
        let cr = ContentRange::parse("bytes 0-1023/4096").unwrap();
        assert_eq!(cr.start, 0);
        assert_eq!(cr.end, 1023);
        assert_eq!(cr.total, 4096);
        assert_eq!(cr.len(), 1024);
        assert!(!cr.is_last());
    }

    #[test]
    fn content_range_rejects_oversize_total() {
        let too_big = format!("bytes 0-10/{}", MAX_UPLOAD_BYTES + 1);
        assert!(ContentRange::parse(&too_big).is_none());
    }

    #[test]
    fn content_range_rejects_end_past_total() {
        assert!(ContentRange::parse("bytes 0-5000/4096").is_none());
    }

    #[test]
    fn sniff_accepts_png_under_size_cap() {
        let bytes = PNG_MAGIC.to_vec();
        let mime = sniff_and_enforce(&bytes, &rules_png()).unwrap();
        assert_eq!(mime, "image/png");
    }

    #[test]
    fn sniff_rejects_disallowed_mime() {
        // JPEG magic — valid MIME, but the rule allows only PNG.
        let bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0];
        let err = sniff_and_enforce(&bytes, &rules_png()).unwrap_err();
        assert!(matches!(err, UploadError::MimeRejected(m) if m.starts_with("image/jpeg")));
    }

    #[test]
    fn sniff_rejects_oversize() {
        let bytes = vec![0u8; 2048];
        let err = sniff_and_enforce(&bytes, &rules_png()).unwrap_err();
        assert!(matches!(err, UploadError::TooLarge));
    }

    #[test]
    fn chunk_store_assembles_in_order() {
        let store = ChunkedUploadStore::new();
        let sess = Uuid::new_v4();
        let r1 = ContentRange::parse("bytes 0-3/8").unwrap();
        let r2 = ContentRange::parse("bytes 4-7/8").unwrap();
        assert!(store.append(sess, r1, &[1, 2, 3, 4]).unwrap().is_none());
        let done = store.append(sess, r2, &[5, 6, 7, 8]).unwrap().unwrap();
        assert_eq!(done, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn chunk_store_rejects_gap() {
        let store = ChunkedUploadStore::new();
        let sess = Uuid::new_v4();
        let r1 = ContentRange::parse("bytes 0-3/8").unwrap();
        // Gap: this chunk claims start=5 but first chunk ended at 3.
        let r2 = ContentRange::parse("bytes 5-7/8").unwrap();
        store.append(sess, r1, &[1, 2, 3, 4]).unwrap();
        assert!(matches!(
            store.append(sess, r2, &[6, 7, 8]),
            Err(UploadError::BadRange)
        ));
    }

    #[tokio::test]
    async fn finalize_hashes_and_stores() {
        let storage = InMemoryStorage::default();
        let key = make_storage_key(Uuid::nil(), Uuid::nil(), Uuid::nil(), "cat.png");
        assert!(key.starts_with("forms/00000000"));
        let bytes = PNG_MAGIC.to_vec();
        let stored = finalize_upload(&storage, &key, &bytes, &rules_png())
            .await
            .unwrap();
        assert_eq!(stored.mime_type, "image/png");
        assert_eq!(stored.size_bytes, PNG_MAGIC.len() as i64);
        assert_eq!(storage.head(&key).await.unwrap(), Some(PNG_MAGIC.len() as u64));
    }
}
