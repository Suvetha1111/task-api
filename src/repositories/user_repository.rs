use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{NewUser, UserRow};

#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserRow>, AppError>;
    /// Insert the user if the email does not exist. Returns `true` when a row was created.
    async fn insert_if_missing(&self, user: &NewUser) -> Result<bool, AppError>;
}

#[derive(Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const USER_COLUMNS: &str = "id, full_name, email, hashed_password, role, created_at, updated_at";

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users WHERE email = $1"
        ))
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserRow>, AppError> {
        let row = sqlx::query_as::<_, UserRow>(&format!(
            "SELECT {USER_COLUMNS} FROM users WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn insert_if_missing(&self, user: &NewUser) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT INTO users (full_name, email, hashed_password, role)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (email) DO NOTHING",
        )
        .bind(&user.full_name)
        .bind(&user.email)
        .bind(&user.hashed_password)
        .bind(user.role.as_str())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
