use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppError;
use crate::models::SeedResponse;
use crate::services::UserService;

#[async_trait]
pub trait SeedManager: Send + Sync + 'static {
    async fn seed_users(&self) -> Result<SeedResponse, AppError>;
}

pub struct SeedManagerImpl {
    users: Arc<dyn UserService>,
}

impl SeedManagerImpl {
    pub fn new(users: Arc<dyn UserService>) -> Self {
        Self { users }
    }
}

#[async_trait]
impl SeedManager for SeedManagerImpl {
    async fn seed_users(&self) -> Result<SeedResponse, AppError> {
        self.users.seed_users().await
    }
}
