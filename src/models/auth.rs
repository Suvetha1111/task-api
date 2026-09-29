use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::models::user::Role;

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "email must be a valid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "password must not be empty"))]
    pub password: String,
}

/// Login never returns a token – only the id of the pending 2FA challenge.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LoginResponse {
    pub login_challenge_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct Verify2faRequest {
    pub login_challenge_id: Uuid,
    #[validate(length(equal = 6, message = "code must be 6 digits"))]
    pub code: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
}

/// JWT claims (HS256).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// User id.
    pub sub: Uuid,
    pub email: String,
    pub role: Role,
    pub exp: i64,
    pub iat: i64,
}

/// Database row for `login_challenges`. Holds only a bcrypt hash of the code.
#[derive(Debug, Clone, FromRow)]
pub struct LoginChallengeRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewLoginChallenge {
    pub user_id: Uuid,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewEmailLog {
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub verification_code: String,
}

/// Database row for `email_logs` (local development inbox).
#[derive(Debug, Clone, FromRow)]
pub struct EmailLogRow {
    pub id: Uuid,
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub verification_code: String,
    pub created_at: DateTime<Utc>,
}

/// Response of `GET /dev/email-logs/latest`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EmailLogResponse {
    pub to_email: String,
    pub subject: String,
    pub verification_code: String,
    pub created_at: DateTime<Utc>,
}

impl From<EmailLogRow> for EmailLogResponse {
    fn from(row: EmailLogRow) -> Self {
        Self {
            to_email: row.to_email,
            subject: row.subject,
            verification_code: row.verification_code,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ResetResponse {
    pub reset: bool,
}
