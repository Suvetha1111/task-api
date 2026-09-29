use std::env;

use crate::error::AppError;

/// Runtime configuration, read from environment variables (see `.env.example`).
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_secs: u64,
    pub otp_ttl_seconds: u64,
    pub app_env: String,
    pub server_host: String,
    pub server_port: u16,
}

impl Config {
    /// Load from the process environment. Required: `DATABASE_URL`, `REDIS_URL`,
    /// `JWT_SECRET`. Everything else has a sensible default.
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            redis_url: required("REDIS_URL")?,
            jwt_secret: required("JWT_SECRET")?,
            jwt_expiry_secs: parsed_or("JWT_EXPIRY_SECS", 3600)?,
            otp_ttl_seconds: parsed_or("OTP_TTL_SECONDS", 300)?,
            app_env: env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: parsed_or("SERVER_PORT", 8080)?,
        })
    }

    pub fn is_development(&self) -> bool {
        self.app_env == "development"
    }
}

fn required(key: &str) -> Result<String, AppError> {
    env::var(key).map_err(|_| AppError::Internal(format!("missing required env var {key}")))
}

fn parsed_or<T: std::str::FromStr>(key: &str, default: T) -> Result<T, AppError> {
    match env::var(key) {
        Ok(raw) => raw
            .parse::<T>()
            .map_err(|_| AppError::Internal(format!("env var {key} is not a valid number"))),
        Err(_) => Ok(default),
    }
}
