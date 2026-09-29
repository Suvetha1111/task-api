use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppError;
use crate::models::EmailLogResponse;
use crate::repositories::EmailLogRepository;

#[async_trait]
pub trait DevManager: Send + Sync + 'static {
    /// The most recent "email" in the local inbox. 404 outside development.
    async fn latest_email_log(&self) -> Result<EmailLogResponse, AppError>;
}

pub struct DevManagerImpl {
    email_logs: Arc<dyn EmailLogRepository>,
    enabled: bool,
}

impl DevManagerImpl {
    pub fn new(email_logs: Arc<dyn EmailLogRepository>, enabled: bool) -> Self {
        Self {
            email_logs,
            enabled,
        }
    }
}

#[async_trait]
impl DevManager for DevManagerImpl {
    async fn latest_email_log(&self) -> Result<EmailLogResponse, AppError> {
        if !self.enabled {
            return Err(AppError::NotFound("not found".to_string()));
        }
        self.email_logs
            .latest()
            .await?
            .map(EmailLogResponse::from)
            .ok_or_else(|| AppError::NotFound("no email logs yet".to_string()))
    }
}
