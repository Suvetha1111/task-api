use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppError;
use crate::models::EmailLogResponse;
use crate::repositories::{EmailLogRepository, LoginChallengeRepository, TaskRepository};
use crate::services::CacheService;

#[async_trait]
pub trait DevManager: Send + Sync + 'static {
    /// The most recent "email" in the local inbox. 404 outside development.
    async fn latest_email_log(&self) -> Result<EmailLogResponse, AppError>;
    /// Drop tasks, challenges, and email logs. Keep users. Clear task caches.
    async fn reset(&self) -> Result<(), AppError>;
}

pub struct DevManagerImpl {
    email_logs: Arc<dyn EmailLogRepository>,
    challenges: Arc<dyn LoginChallengeRepository>,
    tasks: Arc<dyn TaskRepository>,
    cache: Arc<dyn CacheService>,
    enabled: bool,
}

impl DevManagerImpl {
    pub fn new(
        email_logs: Arc<dyn EmailLogRepository>,
        challenges: Arc<dyn LoginChallengeRepository>,
        tasks: Arc<dyn TaskRepository>,
        cache: Arc<dyn CacheService>,
        enabled: bool,
    ) -> Self {
        Self {
            email_logs,
            challenges,
            tasks,
            cache,
            enabled,
        }
    }

    fn require_development(&self) -> Result<(), AppError> {
        if self.enabled {
            Ok(())
        } else {
            Err(AppError::NotFound("not found".to_string()))
        }
    }
}

#[async_trait]
impl DevManager for DevManagerImpl {
    async fn latest_email_log(&self) -> Result<EmailLogResponse, AppError> {
        self.require_development()?;
        self.email_logs
            .latest()
            .await?
            .map(EmailLogResponse::from)
            .ok_or_else(|| AppError::NotFound("no email logs yet".to_string()))
    }

    async fn reset(&self) -> Result<(), AppError> {
        self.require_development()?;
        self.tasks.delete_all().await?;
        self.challenges.delete_all().await?;
        self.email_logs.delete_all().await?;
        self.cache.delete_by_pattern("tasks:view:*").await?;
        Ok(())
    }
}
