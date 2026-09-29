use async_trait::async_trait;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::{EmailLogRow, NewEmailLog};

#[async_trait]
pub trait EmailLogRepository: Send + Sync + 'static {
    async fn insert(&self, log: &NewEmailLog) -> Result<(), AppError>;
    async fn latest(&self) -> Result<Option<EmailLogRow>, AppError>;
}

#[derive(Clone)]
pub struct PgEmailLogRepository {
    pool: PgPool,
}

impl PgEmailLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailLogRepository for PgEmailLogRepository {
    async fn insert(&self, log: &NewEmailLog) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO email_logs (to_email, subject, body, verification_code)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&log.to_email)
        .bind(&log.subject)
        .bind(&log.body)
        .bind(&log.verification_code)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn latest(&self) -> Result<Option<EmailLogRow>, AppError> {
        let row = sqlx::query_as::<_, EmailLogRow>(
            "SELECT id, to_email, subject, body, verification_code, created_at
             FROM email_logs
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }
}
