use actix_web::{web, HttpRequest, HttpResponse};
use uuid::Uuid;
use validator::Validate;

use crate::controllers::auth_user;
use crate::error::AppError;
use crate::models::{AssignTasksRequest, CreateTaskRequest, PatchTaskRequest};
use crate::state::AppState;

/// Create a task. Admin only.
#[utoipa::path(
    post,
    path = "/tasks",
    tag = "tasks",
    security(("bearer" = [])),
    request_body = CreateTaskRequest,
    responses(
        (status = 201, body = TaskResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Staff may not create tasks")
    )
)]
pub async fn create_task(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = auth_user(&req)?;
    let payload = body.into_inner();
    payload.validate()?;
    let task = state
        .tasks
        .task_manager
        .create_task(&caller, payload)
        .await?;
    Ok(HttpResponse::Created().json(task))
}

/// Assign tasks to a user by email. Admin only.
#[utoipa::path(
    post,
    path = "/tasks/assign",
    tag = "tasks",
    security(("bearer" = [])),
    request_body = AssignTasksRequest,
    responses(
        (status = 200, body = AssignTasksResponse),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Staff may not assign tasks"),
        (status = 404, description = "Unknown task id or assignee email")
    )
)]
pub async fn assign_tasks(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<AssignTasksRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = auth_user(&req)?;
    let payload = body.into_inner();
    payload.validate()?;
    let result = state
        .tasks
        .task_manager
        .assign_tasks(&caller, payload)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Update any subset of a task's fields. Admin only.
#[utoipa::path(
    patch,
    path = "/tasks/{id}",
    tag = "tasks",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Task id")),
    request_body = PatchTaskRequest,
    responses(
        (status = 200, body = TaskResponse),
        (status = 400, description = "Empty patch"),
        (status = 401, description = "Missing or invalid token"),
        (status = 403, description = "Staff may not update tasks"),
        (status = 404, description = "Unknown task or assignee")
    )
)]
pub async fn patch_task(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<PatchTaskRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = auth_user(&req)?;
    let payload = body.into_inner();
    payload.validate()?;
    let task = state
        .tasks
        .task_manager
        .patch_task(&caller, path.into_inner(), payload)
        .await?;
    Ok(HttpResponse::Ok().json(task))
}

/// Tasks assigned to the caller, served from Redis when cached.
#[utoipa::path(
    get,
    path = "/tasks/view-my-tasks",
    tag = "tasks",
    security(("bearer" = [])),
    responses(
        (status = 200, body = MyTasksResponse),
        (status = 401, description = "Missing or invalid token")
    )
)]
pub async fn view_my_tasks(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let caller = auth_user(&req)?;
    let result = state.tasks.task_manager.view_my_tasks(&caller).await?;
    Ok(HttpResponse::Ok().json(result))
}
