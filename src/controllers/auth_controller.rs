use actix_web::{web, HttpResponse};
use validator::Validate;

use crate::error::AppError;
use crate::models::{LoginRequest, Verify2faRequest};
use crate::state::AppState;

/// Step 1: check email + password and start a 2FA challenge. Never returns a token.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, body = LoginResponse),
        (status = 401, description = "Invalid email or password")
    )
)]
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();
    req.validate()?;
    let result = state.auth.auth_manager.login(req).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Step 2: exchange the challenge id and 6-digit code for a JWT.
#[utoipa::path(
    post,
    path = "/auth/verify-2fa",
    tag = "auth",
    request_body = Verify2faRequest,
    responses(
        (status = 200, body = TokenResponse),
        (status = 401, description = "Unknown, used, expired or incorrect code")
    )
)]
pub async fn verify_2fa(
    state: web::Data<AppState>,
    body: web::Json<Verify2faRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();
    req.validate()?;
    let result = state.auth.auth_manager.verify_2fa(req).await?;
    Ok(HttpResponse::Ok().json(result))
}
