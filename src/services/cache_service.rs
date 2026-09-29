use async_trait::async_trait;
use deadpool_redis::{redis::AsyncCommands, Pool};
use uuid::Uuid;

use crate::error::AppError;

/// TTL of the "my tasks" view in Redis.
pub const TASKS_VIEW_TTL_SECS: u64 = 60;

/// Redis key for a user's task view: `tasks:view:{user_id}`.
pub fn tasks_view_key(user_id: Uuid) -> String {
    format!("tasks:view:{user_id}")
}

/// Thin, typed wrapper over Redis. Any Redis failure surfaces as
/// `AppError::Redis` (HTTP 500) – the cache is never silently skipped.
#[async_trait]
pub trait CacheService: Send + Sync + 'static {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError>;
    async fn set_ex(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), AppError>;
    async fn delete(&self, keys: &[String]) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct RedisCacheService {
    pool: Pool,
}

impl RedisCacheService {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CacheService for RedisCacheService {
    async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.pool.get().await?;
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    async fn set_ex(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), AppError> {
        let mut conn = self.pool.get().await?;
        conn.set_ex::<_, _, ()>(key, value, ttl_secs).await?;
        Ok(())
    }

    async fn delete(&self, keys: &[String]) -> Result<(), AppError> {
        if keys.is_empty() {
            return Ok(());
        }
        let mut conn = self.pool.get().await?;
        conn.del::<_, ()>(keys).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_format() {
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(
            tasks_view_key(id),
            "tasks:view:11111111-2222-3333-4444-555555555555"
        );
    }

    #[test]
    fn ttl_is_sixty_seconds() {
        assert_eq!(TASKS_VIEW_TTL_SECS, 60);
    }
}
