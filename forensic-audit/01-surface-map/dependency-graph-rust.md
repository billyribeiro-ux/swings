# Rust Module Dependency Graph — Phase 1 Surface Map

One section per file under `backend/src/handlers/` and `backend/src/services/`. Each section lists the verbatim `use` statements (single-line only — multi-line `use { ... }` is rendered on the opening line; descend into the file for the full path list).

## `backend/src/handlers/admin.rs`

```rust
use axum::{;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/admin_audit.rs`

```rust
use std::fmt::Write as _;
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/admin_consent.rs`

```rust
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/admin_dsar.rs`

```rust
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/admin_impersonation.rs`

```rust
use axum::{;
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/admin_ip_allowlist.rs`

```rust
use axum::{;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/admin_members.rs`

```rust
use argon2::{;
use axum::{;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/admin_orders.rs`

```rust
use std::fmt::Write as _;
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/admin_roles.rs`

```rust
use std::collections::BTreeSet;
use axum::{;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::{;
```

## `backend/src/handlers/admin_security.rs`

```rust
use axum::{;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
use super::*;
```

## `backend/src/handlers/admin_settings.rs`

```rust
use axum::{;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::{IntoParams, ToSchema};
use crate::{;
```

## `backend/src/handlers/admin_subscriptions.rs`

```rust
use axum::{;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/analytics.rs`

```rust
use axum::{extract::State, routing::post, Json, Router};
use crate::{;
```

## `backend/src/handlers/auth.rs`

```rust
use argon2::{;
use axum::{;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use time::Duration as CookieDuration;
use uuid::Uuid;
use validator::Validate;
use crate::{;
use base64::Engine;
use super::*;
use axum::http::{HeaderMap, HeaderValue};
use base64::Engine;
```

## `backend/src/handlers/blog.rs`

```rust
use argon2::{;
use axum::{;
use bytes::Bytes;
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/cart.rs`

```rust
use axum::{;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/catalog.rs`

```rust
use axum::{;
use serde::Deserialize;
use crate::commerce::catalog::{self, Category, SearchParams, SearchResponse};
use crate::error::AppResult;
use crate::AppState;
```

## `backend/src/handlers/consent.rs`

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use crate::{;
use super::*;
```

## `backend/src/handlers/coupons.rs`

```rust
use axum::{;
use chrono::Utc;
use rand::Rng;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/courses.rs`

```rust
use axum::{;
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/csp_report.rs`

```rust
use axum::{;
use serde::Deserialize;
use crate::{;
use super::*;
use axum::http::HeaderValue;
```

## `backend/src/handlers/forms.rs`

```rust
use async_trait::async_trait;
use axum::{;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
use super::*;
```

## `backend/src/handlers/health.rs`

```rust
use std::time::Duration;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use crate::{;
```

## `backend/src/handlers/member.rs`

```rust
use argon2::{;
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/mod.rs`

_(no use statements at module scope)_

## `backend/src/handlers/notifications.rs`

```rust
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/outbox.rs`

```rust
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use crate::{;
```

## `backend/src/handlers/popups.rs`

```rust
use axum::{;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use crate::{;
use crate::popups::{;
use super::*;
```

## `backend/src/handlers/pricing.rs`

```rust
use axum::{;
use uuid::Uuid;
use validator::Validate;
use crate::{;
```

## `backend/src/handlers/products.rs`

```rust
use axum::{;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::{;
use crate::commerce::products::{BundleItem, DownloadableAsset, Product, ProductVariant};
```

## `backend/src/handlers/webhooks.rs`

```rust
use axum::{;
use chrono::DateTime;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;
use crate::{;
use subtle::ConstantTimeEq;
use super::verify_stripe_signature;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
```

## `backend/src/services/audit.rs`

```rust
use std::net::IpAddr;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;
use crate::error::{AppError, AppResult};
use crate::extractors::{AdminUser, AuthUser, ClientInfo, PrivilegedUser};
use crate::models::UserRole;
use super::*;
```

## `backend/src/services/audit_retention.rs`

```rust
use std::time::Duration;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::time::Instant;
use crate::settings::Cache as SettingsCache;
use super::*;
```

## `backend/src/services/blog_scheduler.rs`

```rust
use std::time::{Duration, Instant};
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;
```

## `backend/src/services/dsar_admin.rs`

```rust
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::consent::{dsar_export::DsarExport, records::DsarRow};
use super::*;
```

## `backend/src/services/dsar_artifact_sweep.rs`

```rust
use std::time::Duration;
use sqlx::{PgPool, Row};
use tokio::sync::broadcast;
use tokio::time::Instant;
use uuid::Uuid;
use crate::services::MediaBackend;
use super::*;
```

## `backend/src/services/dsar_worker.rs`

```rust
use std::time::Duration;
use bytes::Bytes;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::time::Instant;
use uuid::Uuid;
use crate::services::{audit, dsar_admin, MediaBackend};
use super::*;
```

## `backend/src/services/idempotency_gc.rs`

```rust
use std::time::{Duration, Instant};
use sqlx::PgPool;
use tokio::sync::broadcast;
use crate::settings::Cache as SettingsCache;
use super::*;
```

## `backend/src/services/mod.rs`

```rust
pub use audit::{;
pub use storage::{MediaBackend, R2Storage, StorageError};
```

## `backend/src/services/pricing_rollout.rs`

```rust
use crate::{;
use super::*;
use crate::models::SubscriptionPlan;
use chrono::Utc;
use uuid::Uuid;
```

## `backend/src/services/storage.rs`

```rust
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use bytes::Bytes;
use std::env;
use super::*;
```

