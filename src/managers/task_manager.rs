use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    AssignTasksRequest, AssignTasksResponse, AuthUser, CreateTaskRequest, MyTasksResponse,
    PatchTaskRequest, TaskResponse,
};
use crate::services::TaskService;

#[async_trait]
pub trait TaskManager: Send + Sync + 'static {
    async fn create_task(
        &self,
        caller: &AuthUser,
        req: CreateTaskRequest,
    ) -> Result<TaskResponse, AppError>;
    async fn assign_tasks(
        &self,
        caller: &AuthUser,
        req: AssignTasksRequest,
    ) -> Result<AssignTasksResponse, AppError>;
    async fn patch_task(
        &self,
        caller: &AuthUser,
        task_id: Uuid,
        req: PatchTaskRequest,
    ) -> Result<TaskResponse, AppError>;
    async fn view_my_tasks(&self, caller: &AuthUser) -> Result<MyTasksResponse, AppError>;
}

pub struct TaskManagerImpl {
    tasks: Arc<dyn TaskService>,
}

impl TaskManagerImpl {
    pub fn new(tasks: Arc<dyn TaskService>) -> Self {
        Self { tasks }
    }
}

#[async_trait]
impl TaskManager for TaskManagerImpl {
    async fn create_task(
        &self,
        caller: &AuthUser,
        req: CreateTaskRequest,
    ) -> Result<TaskResponse, AppError> {
        self.tasks.create(caller, req).await
    }

    async fn assign_tasks(
        &self,
        caller: &AuthUser,
        req: AssignTasksRequest,
    ) -> Result<AssignTasksResponse, AppError> {
        self.tasks.assign(caller, req).await
    }

    async fn patch_task(
        &self,
        caller: &AuthUser,
        task_id: Uuid,
        req: PatchTaskRequest,
    ) -> Result<TaskResponse, AppError> {
        self.tasks.patch(caller, task_id, req).await
    }

    async fn view_my_tasks(&self, caller: &AuthUser) -> Result<MyTasksResponse, AppError> {
        self.tasks.view_my_tasks(caller).await
    }
}
