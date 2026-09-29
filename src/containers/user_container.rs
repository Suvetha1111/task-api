use std::sync::Arc;

use sqlx::PgPool;

use crate::managers::{SeedManager, SeedManagerImpl};
use crate::repositories::{PgUserRepository, UserRepository};
use crate::services::UserServiceImpl;

#[derive(Clone)]
pub struct UserContainer {
    pub user_repository: Arc<dyn UserRepository>,
    pub seed_manager: Arc<dyn SeedManager>,
}

impl UserContainer {
    pub fn new(pool: &PgPool) -> Self {
        let user_repository: Arc<dyn UserRepository> =
            Arc::new(PgUserRepository::new(pool.clone()));
        let user_service = Arc::new(UserServiceImpl::new(user_repository.clone()));
        Self {
            user_repository,
            seed_manager: Arc::new(SeedManagerImpl::new(user_service)),
        }
    }
}
