use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::error::AppError;
use crate::models::user::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "todo" => Ok(TaskStatus::Todo),
            "in_progress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            other => Err(AppError::Internal(format!(
                "unknown status in database: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "low" => Ok(Priority::Low),
            "medium" => Ok(Priority::Medium),
            "high" => Ok(Priority::High),
            other => Err(AppError::Internal(format!(
                "unknown priority in database: {other}"
            ))),
        }
    }
}

/// Database row for `tasks`.
#[derive(Debug, Clone, FromRow)]
pub struct TaskRow {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub created_by_id: Uuid,
    pub assigned_to_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row returned by the "my tasks" query (task joined with assignee email).
#[derive(Debug, Clone, FromRow)]
pub struct AssignedTaskRow {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: String,
}

/// Full task representation returned by create / patch.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TaskResponse {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub created_by_id: Uuid,
    pub assigned_to_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TaskResponse {
    pub fn from_row(row: TaskRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            title: row.title,
            description: row.description,
            status: TaskStatus::parse(&row.status)?,
            priority: Priority::parse(&row.priority)?,
            created_by_id: row.created_by_id,
            assigned_to_id: row.assigned_to_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateTaskRequest {
    #[validate(length(min = 1, max = 200, message = "title must be 1-200 characters"))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 5000, message = "description must be at most 5000 characters"))]
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct AssignTasksRequest {
    #[validate(length(min = 1, message = "task_ids must not be empty"))]
    pub task_ids: Vec<Uuid>,
    #[validate(email(message = "assignee_email must be a valid email"))]
    pub assignee_email: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssignTasksResponse {
    pub assigned: usize,
    pub assignee_email: String,
    pub task_ids: Vec<Uuid>,
}

/// Any subset of fields may be present.
#[derive(Debug, Clone, Default, Deserialize, Validate, ToSchema)]
pub struct PatchTaskRequest {
    #[validate(length(min = 1, max = 200, message = "title must be 1-200 characters"))]
    pub title: Option<String>,
    #[validate(length(max = 5000, message = "description must be at most 5000 characters"))]
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    #[validate(email(message = "assignee_email must be a valid email"))]
    pub assignee_email: Option<String>,
}

impl PatchTaskRequest {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.status.is_none()
            && self.priority.is_none()
            && self.assignee_email.is_none()
    }
}

/// Column-level update passed to the repository (assignee already resolved).
#[derive(Debug, Clone, Default)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub assigned_to_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// GET /tasks/view-my-tasks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct ViewUser {
    pub email: String,
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct ViewTask {
    pub id: Uuid,
    pub title: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub assigned_to: String,
}

impl ViewTask {
    pub fn from_row(row: AssignedTaskRow) -> Result<Self, AppError> {
        Ok(Self {
            id: row.id,
            title: row.title,
            status: TaskStatus::parse(&row.status)?,
            priority: Priority::parse(&row.priority)?,
            assigned_to: row.assigned_to,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct ViewSummary {
    pub total_assigned_tasks: usize,
}

/// Exactly what is stored in Redis. `cache.hit` is deliberately not part of it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct MyTasksPayload {
    pub user: ViewUser,
    pub tasks: Vec<ViewTask>,
    pub summary: ViewSummary,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CacheInfo {
    pub hit: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MyTasksResponse {
    pub user: ViewUser,
    pub tasks: Vec<ViewTask>,
    pub summary: ViewSummary,
    pub cache: CacheInfo,
}

impl MyTasksResponse {
    pub fn new(payload: MyTasksPayload, hit: bool) -> Self {
        Self {
            user: payload.user,
            tasks: payload.tasks,
            summary: payload.summary,
            cache: CacheInfo { hit },
        }
    }
}
