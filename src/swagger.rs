use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::controllers::{auth_controller, dev_controller, seed_controller, task_controller};
use crate::models;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Task API", description = "Task management with 2FA login and Redis-cached views"),
    paths(
        seed_controller::seed_users,
        auth_controller::login,
        auth_controller::verify_2fa,
        dev_controller::latest_email_log,
        dev_controller::reset,
        task_controller::create_task,
        task_controller::assign_tasks,
        task_controller::patch_task,
        task_controller::view_my_tasks,
    ),
    components(schemas(
        models::Role,
        models::TaskStatus,
        models::Priority,
        models::SeedResponse,
        models::LoginRequest,
        models::LoginResponse,
        models::Verify2faRequest,
        models::TokenResponse,
        models::EmailLogResponse,
        models::ResetResponse,
        models::CreateTaskRequest,
        models::AssignTasksRequest,
        models::AssignTasksResponse,
        models::PatchTaskRequest,
        models::TaskResponse,
        models::ViewUser,
        models::ViewTask,
        models::ViewSummary,
        models::CacheInfo,
        models::MyTasksResponse,
    )),
    modifiers(&BearerAuth),
    tags(
        (name = "seed"), (name = "auth"), (name = "dev"), (name = "tasks")
    )
)]
pub struct ApiDoc;
