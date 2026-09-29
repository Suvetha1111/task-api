use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use rand::{rngs::OsRng, Rng};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{LoginResponse, NewEmailLog, NewLoginChallenge, TokenResponse};
use crate::repositories::{EmailLogRepository, LoginChallengeRepository, UserRepository};
use crate::services::user_service::BCRYPT_COST;
use crate::services::TokenService;

const INVALID_CREDENTIALS: &str = "invalid email or password";
const EMAIL_SUBJECT: &str = "Your verification code";

#[async_trait]
pub trait AuthService: Send + Sync + 'static {
    /// Step 1 of login: check credentials, create a challenge, "send" the code.
    async fn login(&self, email: &str, password: &str) -> Result<LoginResponse, AppError>;
    /// Step 2 of login: consume the challenge and issue a JWT.
    async fn verify_2fa(&self, challenge_id: Uuid, code: &str) -> Result<TokenResponse, AppError>;
}

pub struct AuthServiceImpl {
    pool: PgPool,
    users: Arc<dyn UserRepository>,
    challenges: Arc<dyn LoginChallengeRepository>,
    email_logs: Arc<dyn EmailLogRepository>,
    tokens: Arc<dyn TokenService>,
    otp_ttl_seconds: u64,
    /// A bcrypt hash of a throwaway password, verified when the email is unknown
    /// so a login attempt takes the same time whether or not the user exists.
    dummy_hash: String,
}

impl AuthServiceImpl {
    pub fn new(
        pool: PgPool,
        users: Arc<dyn UserRepository>,
        challenges: Arc<dyn LoginChallengeRepository>,
        email_logs: Arc<dyn EmailLogRepository>,
        tokens: Arc<dyn TokenService>,
        otp_ttl_seconds: u64,
    ) -> Result<Self, AppError> {
        Ok(Self {
            pool,
            users,
            challenges,
            email_logs,
            tokens,
            otp_ttl_seconds,
            dummy_hash: bcrypt::hash("not-a-real-password", BCRYPT_COST)?,
        })
    }
}

/// Cryptographically secure 6-digit code, zero-padded.
pub fn generate_code() -> String {
    let n: u32 = OsRng.gen_range(0..1_000_000);
    format!("{n:06}")
}

/// When a challenge created at `now` stops being valid.
pub fn challenge_expiry(now: chrono::DateTime<Utc>, ttl_seconds: u64) -> chrono::DateTime<Utc> {
    now + Duration::seconds(i64::try_from(ttl_seconds).unwrap_or(i64::MAX))
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn login(&self, email: &str, password: &str) -> Result<LoginResponse, AppError> {
        let user = self.users.find_by_email(email).await?;

        // Verify against the real hash, or a dummy one, so the response time
        // does not reveal whether the email exists.
        let hash = user
            .as_ref()
            .map(|u| u.hashed_password.as_str())
            .unwrap_or(self.dummy_hash.as_str());
        let password_ok = bcrypt::verify(password, hash)?;

        let user = match user {
            Some(u) if password_ok => u,
            _ => return Err(AppError::Unauthorized(INVALID_CREDENTIALS.to_string())),
        };

        let code = generate_code();
        let now = Utc::now();
        let expires_at = challenge_expiry(now, self.otp_ttl_seconds);

        let challenge_id = self
            .challenges
            .insert(&NewLoginChallenge {
                user_id: user.id,
                code_hash: bcrypt::hash(&code, BCRYPT_COST)?,
                expires_at,
            })
            .await?;

        self.email_logs
            .insert(&NewEmailLog {
                to_email: user.email.clone(),
                subject: EMAIL_SUBJECT.to_string(),
                body: format!(
                    "Your verification code is {code}. It expires in {} seconds.",
                    self.otp_ttl_seconds
                ),
                verification_code: code.clone(),
            })
            .await?;

        // The only place the raw code is logged. Passwords are never logged.
        tracing::info!(
            email = %user.email,
            login_challenge_id = %challenge_id,
            verification_code = %code,
            "2FA code issued"
        );

        Ok(LoginResponse {
            login_challenge_id: challenge_id,
        })
    }

    async fn verify_2fa(&self, challenge_id: Uuid, code: &str) -> Result<TokenResponse, AppError> {
        let mut tx = self.pool.begin().await?;

        let challenge = self
            .challenges
            .find_for_update(&mut tx, challenge_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("unknown login challenge".to_string()))?;

        if challenge.used_at.is_some() {
            return Err(AppError::Unauthorized(
                "verification code has already been used".to_string(),
            ));
        }

        let now = Utc::now();
        if now >= challenge.expires_at {
            return Err(AppError::Unauthorized(
                "verification code has expired".to_string(),
            ));
        }

        if !bcrypt::verify(code, &challenge.code_hash)? {
            // Transaction is dropped here -> rolled back; the challenge stays usable.
            return Err(AppError::Unauthorized(
                "incorrect verification code".to_string(),
            ));
        }

        self.challenges
            .mark_used(&mut tx, challenge.id, now)
            .await?;

        let user = self
            .users
            .find_by_id(challenge.user_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("user no longer exists".to_string()))?;

        let access_token = self.tokens.issue(user.id, &user.email, user.role()?)?;

        tx.commit().await?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_is_six_digits() {
        for _ in 0..50 {
            let code = generate_code();
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn expiry_math() {
        let now = Utc::now();
        assert_eq!(challenge_expiry(now, 300), now + Duration::seconds(300));
        assert_eq!(challenge_expiry(now, 1), now + Duration::seconds(1));
    }
}
