use actix_web::{web, HttpResponse};

use crate::error::AppError;
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
