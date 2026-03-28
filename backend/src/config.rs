use std::env;

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
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            jwt_expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .expect("JWT_EXPIRATION_HOURS must be a number"),
            refresh_token_expiration_days: env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .expect("REFRESH_TOKEN_EXPIRATION_DAYS must be a number"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .expect("PORT must be a number"),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            stripe_secret_key: env::var("STRIPE_SECRET_KEY")
                .unwrap_or_default(),
            stripe_webhook_secret: env::var("STRIPE_WEBHOOK_SECRET")
                .unwrap_or_default(),
        }
    }
}
