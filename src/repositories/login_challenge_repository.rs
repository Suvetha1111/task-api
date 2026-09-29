use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{LoginChallengeRow, NewLoginChallenge};
use crate::repositories::Tx;

#[async_trait]
pub trait LoginChallengeRepository: Send + Sync + 'static {
    async fn insert(&self, challenge: &NewLoginChallenge) -> Result<Uuid, AppError>;
    /// Lock the row for the rest of the transaction so two concurrent verify
    /// calls cannot both succeed.
    async fn find_for_update(
        &self,
        tx: &mut Tx<'_>,
        id: Uuid,
    ) -> Result<Option<LoginChallengeRow>, AppError>;
    async fn mark_used(
        &self,
        tx: &mut Tx<'_>,
        id: Uuid,
        used_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct PgLoginChallengeRepository {
    pool: PgPool,
}

impl PgLoginChallengeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LoginChallengeRepository for PgLoginChallengeRepository {
    async fn insert(&self, challenge: &NewLoginChallenge) -> Result<Uuid, AppError> {
        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO login_challenges (user_id, code_hash, expires_at)
             VALUES ($1, $2, $3)
             RETURNING id",
        )
        .bind(challenge.user_id)
        .bind(&challenge.code_hash)
        .bind(challenge.expires_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn find_for_update(
        &self,
        tx: &mut Tx<'_>,
        id: Uuid,
    ) -> Result<Option<LoginChallengeRow>, AppError> {
        let row = sqlx::query_as::<_, LoginChallengeRow>(
            "SELECT id, user_id, code_hash, expires_at, used_at, created_at
             FROM login_challenges
             WHERE id = $1
             FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
        Ok(row)
    }

    async fn mark_used(
        &self,
        tx: &mut Tx<'_>,
        id: Uuid,
        used_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE login_challenges SET used_at = $2 WHERE id = $1")
            .bind(id)
            .bind(used_at)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
