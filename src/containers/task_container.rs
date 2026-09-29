use std::sync::Arc;

use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use crate::managers::{TaskManager, TaskManagerImpl};
use crate::repositories::{PgTaskRepository, PgUserRepository};
use crate::services::{RedisCacheService, TaskServiceImpl};

#[derive(Clone)]
pub struct TaskContainer {
    pub task_manager: Arc<dyn TaskManager>,
}

impl TaskContainer {
    pub fn new(pool: &PgPool, redis: &RedisPool) -> Self {
        let tasks = Arc::new(PgTaskRepository::new(pool.clone()));
        let users = Arc::new(PgUserRepository::new(pool.clone()));
        let cache = Arc::new(RedisCacheService::new(redis.clone()));
        let service = Arc::new(TaskServiceImpl::new(pool.clone(), tasks, users, cache));
        Self {
            task_manager: Arc::new(TaskManagerImpl::new(service)),
        }
    }
}
