use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    AssignTasksRequest, AssignTasksResponse, AuthUser, CreateTaskRequest, MyTasksPayload,
    MyTasksResponse, PatchTaskRequest, TaskResponse, TaskUpdate, ViewSummary, ViewTask, ViewUser,
};
use crate::repositories::task_repository::NewTask;
use crate::repositories::{TaskRepository, UserRepository};
use crate::services::authz::require_admin;
use crate::services::cache_service::{tasks_view_key, TASKS_VIEW_TTL_SECS};
use crate::services::CacheService;

#[async_trait]
pub trait TaskService: Send + Sync + 'static {
    async fn create(
        &self,
        caller: &AuthUser,
        req: CreateTaskRequest,
    ) -> Result<TaskResponse, AppError>;
    async fn assign(
        &self,
        caller: &AuthUser,
        req: AssignTasksRequest,
    ) -> Result<AssignTasksResponse, AppError>;
    async fn patch(
        &self,
        caller: &AuthUser,
        task_id: Uuid,
        req: PatchTaskRequest,
    ) -> Result<TaskResponse, AppError>;
    async fn view_my_tasks(&self, caller: &AuthUser) -> Result<MyTasksResponse, AppError>;
}

pub struct TaskServiceImpl {
    pool: PgPool,
    tasks: Arc<dyn TaskRepository>,
    users: Arc<dyn UserRepository>,
    cache: Arc<dyn CacheService>,
}

impl TaskServiceImpl {
    pub fn new(
        pool: PgPool,
        tasks: Arc<dyn TaskRepository>,
        users: Arc<dyn UserRepository>,
        cache: Arc<dyn CacheService>,
    ) -> Self {
        Self {
            pool,
            tasks,
            users,
            cache,
        }
    }

    /// Delete the cached view of every affected user.
    async fn invalidate(&self, user_ids: impl IntoIterator<Item = Uuid>) -> Result<(), AppError> {
        let keys: Vec<String> = user_ids
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(tasks_view_key)
            .collect();
        self.cache.delete(&keys).await
    }

    async fn resolve_assignee(&self, email: &str) -> Result<Uuid, AppError> {
        self.users
            .find_by_email(email)
            .await?
            .map(|u| u.id)
            .ok_or_else(|| AppError::NotFound(format!("assignee {email} not found")))
    }
}

#[async_trait]
impl TaskService for TaskServiceImpl {
    async fn create(
        &self,
        caller: &AuthUser,
        req: CreateTaskRequest,
    ) -> Result<TaskResponse, AppError> {
        require_admin(caller.role)?;
        let row = self
            .tasks
            .insert(&NewTask {
                title: req.title,
                description: req.description,
                status: req.status.as_str(),
                priority: req.priority.as_str(),
                created_by_id: caller.user_id,
            })
            .await?;
        TaskResponse::from_row(row)
    }

    async fn assign(
        &self,
        caller: &AuthUser,
        req: AssignTasksRequest,
    ) -> Result<AssignTasksResponse, AppError> {
        require_admin(caller.role)?;

        let assignee_id = self.resolve_assignee(&req.assignee_email).await?;
        let ids: Vec<Uuid> = req
            .task_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let mut tx = self.pool.begin().await?;

        let rows = self.tasks.find_by_ids_for_update(&mut tx, &ids).await?;
        if rows.len() != ids.len() {
            let found: BTreeSet<Uuid> = rows.iter().map(|r| r.id).collect();
            let missing = ids
                .iter()
                .find(|id| !found.contains(id))
                .map(|id| id.to_string())
                .unwrap_or_default();
            return Err(AppError::NotFound(format!("task {missing} not found")));
        }

        let previous_assignees: Vec<Uuid> = rows.iter().filter_map(|r| r.assigned_to_id).collect();

        let assigned = self.tasks.assign_many(&mut tx, &ids, assignee_id).await?;
        tx.commit().await?;

        self.invalidate(
            previous_assignees
                .into_iter()
                .chain(std::iter::once(assignee_id)),
        )
        .await?;

        Ok(AssignTasksResponse {
            assigned: usize::try_from(assigned).unwrap_or(ids.len()),
            assignee_email: req.assignee_email,
            task_ids: req.task_ids,
        })
    }

    async fn patch(
        &self,
        caller: &AuthUser,
        task_id: Uuid,
        req: PatchTaskRequest,
    ) -> Result<TaskResponse, AppError> {
        require_admin(caller.role)?;
        if req.is_empty() {
            return Err(AppError::BadRequest(
                "at least one field must be provided".to_string(),
            ));
        }

        let assigned_to_id = match &req.assignee_email {
            Some(email) => Some(self.resolve_assignee(email).await?),
            None => None,
        };

        let mut tx = self.pool.begin().await?;

        let existing = self
            .tasks
            .find_by_ids_for_update(&mut tx, &[task_id])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound(format!("task {task_id} not found")))?;

        let update = TaskUpdate {
            title: req.title,
            description: req.description,
            status: req.status,
            priority: req.priority,
            assigned_to_id,
        };
        let row = self.tasks.update(&mut tx, task_id, &update).await?;
        tx.commit().await?;

        // Both the previous and the new assignee (when any) see a changed view.
        self.invalidate(
            existing
                .assigned_to_id
                .into_iter()
                .chain(row.assigned_to_id),
        )
        .await?;

        TaskResponse::from_row(row)
    }

    async fn view_my_tasks(&self, caller: &AuthUser) -> Result<MyTasksResponse, AppError> {
        let key = tasks_view_key(caller.user_id);

        if let Some(cached) = self.cache.get(&key).await? {
            let payload: MyTasksPayload = serde_json::from_str(&cached)?;
            return Ok(MyTasksResponse::new(payload, true));
        }

        let rows = self.tasks.list_assigned_to(caller.user_id).await?;
        let tasks = rows
            .into_iter()
            .map(ViewTask::from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let payload = MyTasksPayload {
            user: ViewUser {
                email: caller.email.clone(),
                role: caller.role,
            },
            summary: ViewSummary {
                total_assigned_tasks: tasks.len(),
            },
            tasks,
        };

        self.cache
            .set_ex(&key, &serde_json::to_string(&payload)?, TASKS_VIEW_TTL_SECS)
            .await?;

        Ok(MyTasksResponse::new(payload, false))
    }
}
