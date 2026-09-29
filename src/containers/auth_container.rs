use std::sync::Arc;

use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use crate::config::Config;
use crate::error::AppError;
use crate::managers::{AuthManager, AuthManagerImpl, DevManager, DevManagerImpl};
use crate::repositories::{
    PgEmailLogRepository, PgLoginChallengeRepository, PgTaskRepository, PgUserRepository,
    UserRepository,
};
use crate::services::{AuthServiceImpl, RedisCacheService, TokenService};

#[derive(Clone)]
pub struct AuthContainer {
    pub auth_manager: Arc<dyn AuthManager>,
    pub dev_manager: Arc<dyn DevManager>,
}

impl AuthContainer {
    pub fn new(
        pool: &PgPool,
        redis: &RedisPool,
        config: &Config,
        tokens: Arc<dyn TokenService>,
    ) -> Result<Self, AppError> {
        let users: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool.clone()));
        let challenges = Arc::new(PgLoginChallengeRepository::new(pool.clone()));
        let email_logs = Arc::new(PgEmailLogRepository::new(pool.clone()));
        let tasks = Arc::new(PgTaskRepository::new(pool.clone()));
        let cache = Arc::new(RedisCacheService::new(redis.clone()));

        let auth_service = Arc::new(AuthServiceImpl::new(
            pool.clone(),
            users,
            challenges.clone(),
            email_logs.clone(),
            tokens,
            config.otp_ttl_seconds,
        )?);

        Ok(Self {
            auth_manager: Arc::new(AuthManagerImpl::new(auth_service)),
            dev_manager: Arc::new(DevManagerImpl::new(
                email_logs,
                challenges,
                tasks,
                cache,
                config.is_development(),
            )),
        })
    }
}
