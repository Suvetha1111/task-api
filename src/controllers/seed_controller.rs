use actix_web::{web, HttpResponse};

use crate::error::AppError;
use crate::state::AppState;

/// Create the well-known users if they do not exist. Idempotent.
#[utoipa::path(
    post,
    path = "/seed/users",
    tag = "seed",
    responses((status = 200, body = SeedResponse))
)]
pub async fn seed_users(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let result = state.users.seed_manager.seed_users().await?;
    Ok(HttpResponse::Ok().json(result))
}
