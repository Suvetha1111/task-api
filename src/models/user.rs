use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;

/// User role. Serialized as lowercase `admin` / `staff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Staff,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Staff => "staff",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "admin" => Ok(Role::Admin),
            "staff" => Ok(Role::Staff),
            other => Err(AppError::Internal(format!(
                "unknown role in database: {other}"
            ))),
        }
    }
}

/// Database row. Never serialized directly – `hashed_password` must not leak.
#[derive(Debug, Clone, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub hashed_password: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserRow {
    pub fn role(&self) -> Result<Role, AppError> {
        Role::parse(&self.role)
    }
}

/// Values needed to create a user (seed).
#[derive(Debug, Clone)]
pub struct NewUser {
    pub full_name: String,
    pub email: String,
    pub hashed_password: String,
    pub role: Role,
}

/// The identity attached to a request by the auth middleware.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub email: String,
    pub role: Role,
}

/// Public view of a user (no password material).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub role: Role,
}

impl UserSummary {
    pub fn from_row(row: &UserRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            full_name: row.full_name.clone(),
            email: row.email.clone(),
            role: row.role()?,
        })
    }
}

/// Response of `POST /seed/users`.
#[derive(Debug, Serialize, ToSchema)]
pub struct SeedResponse {
    pub created: Vec<String>,
    pub existing: Vec<String>,
}
