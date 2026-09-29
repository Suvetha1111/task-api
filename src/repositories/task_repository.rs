use async_trait::async_trait;
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{AssignedTaskRow, TaskRow, TaskUpdate};
use crate::repositories::Tx;

/// Values for a new task (assignee is always null on create).
#[derive(Debug, Clone)]
pub struct NewTask {
    pub title: String,
    pub description: String,
    pub status: &'static str,
    pub priority: &'static str,
    pub created_by_id: Uuid,
}

#[async_trait]
pub trait TaskRepository: Send + Sync + 'static {
    async fn insert(&self, task: &NewTask) -> Result<TaskRow, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TaskRow>, AppError>;
    /// Fetch the given ids inside a transaction, locking the rows.
    async fn find_by_ids_for_update(
        &self,
        tx: &mut Tx<'_>,
        ids: &[Uuid],
    ) -> Result<Vec<TaskRow>, AppError>;
    async fn assign_many(
        &self,
        tx: &mut Tx<'_>,
        ids: &[Uuid],
        assignee_id: Uuid,
    ) -> Result<u64, AppError>;
    /// Apply a partial update. Returns the updated row.
    async fn update(
        &self,
        tx: &mut Tx<'_>,
        id: Uuid,
        update: &TaskUpdate,
    ) -> Result<TaskRow, AppError>;
    /// Tasks assigned to a user, ordered by `created_at` then `id`.
    async fn list_assigned_to(&self, user_id: Uuid) -> Result<Vec<AssignedTaskRow>, AppError>;
}

#[derive(Clone)]
pub struct PgTaskRepository {
    pool: PgPool,
}

impl PgTaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const TASK_COLUMNS: &str = "id, title, description, status, priority, created_by_id, \
                            assigned_to_id, created_at, updated_at";

#[async_trait]
impl TaskRepository for PgTaskRepository {
    async fn insert(&self, task: &NewTask) -> Result<TaskRow, AppError> {
        let row = sqlx::query_as::<_, TaskRow>(&format!(
            "INSERT INTO tasks (title, description, status, priority, created_by_id)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING {TASK_COLUMNS}"
        ))
        .bind(&task.title)
        .bind(&task.description)
        .bind(task.status)
        .bind(task.priority)
        .bind(task.created_by_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<TaskRow>, AppError> {
        let row = sqlx::query_as::<_, TaskRow>(&format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_by_ids_for_update(
        &self,
        tx: &mut Tx<'_>,
        ids: &[Uuid],
    ) -> Result<Vec<TaskRow>, AppError> {
        let rows = sqlx::query_as::<_, TaskRow>(&format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE id = ANY($1) FOR UPDATE"
        ))
        .bind(ids)
        .fetch_all(&mut **tx)
        .await?;
        Ok(rows)
    }

    async fn assign_many(
        &self,
        tx: &mut Tx<'_>,
        ids: &[Uuid],
        assignee_id: Uuid,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE tasks SET assigned_to_id = $2, updated_at = NOW() WHERE id = ANY($1)",
        )
        .bind(ids)
        .bind(assignee_id)
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }

    async fn update(
        &self,
        tx: &mut Tx<'_>,
        id: Uuid,
        update: &TaskUpdate,
    ) -> Result<TaskRow, AppError> {
        let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new("UPDATE tasks SET ");
        let mut sep = qb.separated(", ");
        if let Some(title) = &update.title {
            sep.push("title = ").push_bind_unseparated(title);
        }
        if let Some(description) = &update.description {
            sep.push("description = ")
                .push_bind_unseparated(description);
        }
        if let Some(status) = update.status {
            sep.push("status = ").push_bind_unseparated(status.as_str());
        }
        if let Some(priority) = update.priority {
            sep.push("priority = ")
                .push_bind_unseparated(priority.as_str());
        }
        if let Some(assignee) = update.assigned_to_id {
            sep.push("assigned_to_id = ")
                .push_bind_unseparated(assignee);
        }
        sep.push("updated_at = NOW()");
        qb.push(" WHERE id = ")
            .push_bind(id)
            .push(format!(" RETURNING {TASK_COLUMNS}"));

        let row = qb.build_query_as::<TaskRow>().fetch_one(&mut **tx).await?;
        Ok(row)
    }

    async fn list_assigned_to(&self, user_id: Uuid) -> Result<Vec<AssignedTaskRow>, AppError> {
        let rows = sqlx::query_as::<_, AssignedTaskRow>(
            "SELECT t.id, t.title, t.status, t.priority, u.email AS assigned_to
             FROM tasks t
             JOIN users u ON u.id = t.assigned_to_id
             WHERE t.assigned_to_id = $1
             ORDER BY t.created_at ASC, t.id ASC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
