use actix_web::{web, HttpResponse};

use crate::error::AppError;
use crate::models::ResetResponse;
use crate::state::AppState;

/// Latest "email" in the local development inbox. 404 unless APP_ENV=development.
#[utoipa::path(
    get,
    path = "/dev/email-logs/latest",
    tag = "dev",
    responses(
        (status = 200, body = EmailLogResponse),
        (status = 404, description = "Not development, or no emails yet")
    )
)]
pub async fn latest_email_log(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let result = state.auth.dev_manager.latest_email_log().await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Clear tasks, challenges, email logs, and task caches. Keeps seeded users.
#[utoipa::path(
    post,
    path = "/dev/reset",
    tag = "dev",
    responses(
        (status = 200, body = ResetResponse),
        (status = 404, description = "Not development")
    )
)]
pub async fn reset(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    state.auth.dev_manager.reset().await?;
    Ok(HttpResponse::Ok().json(ResetResponse { reset: true }))
}
