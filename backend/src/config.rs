use std::env;

use anyhow::{bail, Context, Result};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub refresh_token_expiration_days: i64,
    pub port: u16,
    pub frontend_url: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub upload_dir: String,
    pub api_url: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub app_url: String,
    pub app_env: String,
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    /// Build a [`Config`] from process environment variables. Required variables
    /// fail startup via `anyhow::Error` rather than panicking so `main` can print
    /// a clean error chain.
    pub fn from_env() -> Result<Self> {
        let frontend_url =
            env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| frontend_url.clone())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let jwt_secret = env::var("JWT_SECRET").context("JWT_SECRET must be set")?;

        let jwt_expiration_hours = env::var("JWT_EXPIRATION_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .context("JWT_EXPIRATION_HOURS must be a number")?;
        let refresh_token_expiration_days = env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .context("REFRESH_TOKEN_EXPIRATION_DAYS must be a number")?;
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .context("PORT must be a number")?;
        let smtp_port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .context("SMTP_PORT must be a number")?;

        Ok(Self {
            database_url,
            jwt_secret,
            jwt_expiration_hours,
            refresh_token_expiration_days,
            port,
            frontend_url,
            stripe_secret_key: env::var("STRIPE_SECRET_KEY").unwrap_or_default(),
            stripe_webhook_secret: env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default(),
            upload_dir: env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            api_url: env::var("API_URL").unwrap_or_else(|_| "http://localhost:3001".to_string()),
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            smtp_port,
            smtp_user: env::var("SMTP_USER").unwrap_or_default(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_from: env::var("SMTP_FROM")
                .unwrap_or_else(|_| "noreply@precisionoptionsignals.com".to_string()),
            app_url: env::var("APP_URL").unwrap_or_else(|_| "http://localhost:5173".to_string()),
            app_env: env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            cors_allowed_origins,
        })
    }

    pub fn is_production(&self) -> bool {
        self.app_env.eq_ignore_ascii_case("production")
    }

    /// Returns an error in production when required secrets or URLs are missing or invalid.
    /// Call right after [`Config::from_env`].
    pub fn assert_production_ready(&self) -> Result<()> {
        if !self.is_production() {
            return Ok(());
        }

        let mut missing: Vec<String> = Vec::new();

        if self.database_url.trim().is_empty() {
            missing.push("DATABASE_URL".into());
        }
        if self.jwt_secret.trim().is_empty() {
            missing.push("JWT_SECRET".into());
        }
        if self.api_url.trim().is_empty() {
            missing.push("API_URL".into());
        }
        if self.frontend_url.trim().is_empty() {
            missing.push("FRONTEND_URL".into());
        }
        if self.stripe_secret_key.trim().is_empty() {
            missing.push("STRIPE_SECRET_KEY".into());
        }
        if self.stripe_webhook_secret.trim().is_empty() {
            missing.push("STRIPE_WEBHOOK_SECRET".into());
        }

        if env::var("ADMIN_EMAIL")
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            missing.push("ADMIN_EMAIL".into());
        }
        if env::var("ADMIN_PASSWORD")
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            missing.push("ADMIN_PASSWORD".into());
        }

        if crate::services::R2Storage::from_env().is_err() {
            missing.push(
                "R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET_NAME, R2_PUBLIC_URL"
                    .into(),
            );
        }

        if !missing.is_empty() {
            bail!(
                "APP_ENV=production but required configuration is missing or invalid:\n  - {}",
                missing.join("\n  - ")
            );
        }

        Ok(())
    }
}
