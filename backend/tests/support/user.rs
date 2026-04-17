//! User seeding + JWT minting for integration tests.
//!
//! The production `register` / `login` flows are exercised against real HTTP
//! requests in end-to-end tests, but many subsystem tests only care about the
//! *authenticated-as-X* precondition. `seed_user` / `seed_admin` bypass HTTP
//! entirely and install a row directly, mirroring the password-hash and
//! refresh-token shape that `handlers::auth` would have produced.
//!
//! All tokens are signed with the same `jwt_secret` that the harness's
//! `AppState` hands to its `Router`, so the resulting `access_token` passes
//! the production `AuthUser` extractor unchanged.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::{TestAppError, TestResult};

/// User role stored in the `users.role` column.
///
/// Mirrors `swings_api::models::UserRole` without taking a public dependency
/// on the enum — tests that need the real type can still reach for it via
/// `use swings_api::models::UserRole;`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestRole {
    Member,
    Admin,
}

impl TestRole {
    pub(crate) fn as_sql(self) -> &'static str {
        match self {
            TestRole::Member => "member",
            TestRole::Admin => "admin",
        }
    }

    pub(crate) fn as_claim(self) -> &'static str {
        match self {
            TestRole::Member => "member",
            TestRole::Admin => "admin",
        }
    }
}

/// Test fixture representing a freshly seeded user.
///
/// The clear-text `password` is retained on purpose so callers can drive a
/// `POST /api/auth/login` roundtrip when the flow under test is the password
/// verification itself.
#[derive(Debug, Clone)]
pub struct TestUser {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub name: String,
    pub role: TestRole,
    pub access_token: String,
    pub refresh_token: String,
}

/// JWT claims that must byte-identical-match `swings_api::extractors::Claims`.
///
/// We mirror the struct here instead of depending on the crate-internal
/// `Claims` type to keep the harness decoupled from private-ish symbols —
/// the shape only changes when the auth contract changes, which should be a
/// deliberate coordinated update.
#[derive(serde::Serialize)]
struct HarnessClaims {
    sub: Uuid,
    role: String,
    iat: usize,
    exp: usize,
}

/// Seed a user directly into the schema-scoped pool and return a fresh
/// [`TestUser`] with a matching access token + refresh token.
///
/// This calls the same Argon2 defaults as the real registration handler, so
/// a subsequent `POST /api/auth/login` with `password` succeeds.
pub(crate) async fn seed(
    pool: &PgPool,
    jwt_secret: &str,
    jwt_expiration_hours: i64,
    refresh_token_expiration_days: i64,
    role: TestRole,
) -> TestResult<TestUser> {
    let user_id = Uuid::new_v4();
    let suffix = Uuid::new_v4().simple().to_string();
    let email = format!("test-{suffix}@example.test");
    let password = format!("pw-{suffix}");
    let name = match role {
        TestRole::Admin => format!("Test Admin {}", &suffix[..8]),
        TestRole::Member => format!("Test Member {}", &suffix[..8]),
    };

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| TestAppError::Auth(format!("argon2 hash: {e}")))?
        .to_string();

    // Insert matching the column set from `db::create_user` / `db::seed_admin`.
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, role)
         VALUES ($1, $2, $3, $4, $5::user_role)",
    )
    .bind(user_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(&name)
    .bind(role.as_sql())
    .execute(pool)
    .await?;

    // Mint a matching JWT access token and a refresh token row.
    let now = Utc::now();
    let claims = HarnessClaims {
        sub: user_id,
        role: role.as_claim().to_string(),
        iat: usize::try_from(now.timestamp().max(0)).unwrap_or(0),
        exp: usize::try_from(
            (now + Duration::hours(jwt_expiration_hours))
                .timestamp()
                .max(0),
        )
        .unwrap_or(0),
    };
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| TestAppError::Auth(format!("jwt encode: {e}")))?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = sha256_hex(&refresh_token);
    let expires_at = now + Duration::days(refresh_token_expiration_days);
    let family_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, family_id, used)
         VALUES ($1, $2, $3, $4, $5, FALSE)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .bind(family_id)
    .execute(pool)
    .await?;

    Ok(TestUser {
        id: user_id,
        email,
        password,
        name,
        role,
        access_token,
        refresh_token,
    })
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
