//! Bearer-token authentication for protected routes.
//!
//! Unlike an "optional auth" middleware that logs and continues, this one
//! rejects with 401 whenever the token is missing, malformed, has a bad
//! signature, is expired, or refers to a user that no longer exists.

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header,
    middleware::Next,
    web::Data,
    Error, HttpMessage,
};

use crate::error::AppError;
use crate::models::AuthUser;
use crate::state::AppState;

pub async fn require_auth<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error>
where
    B: MessageBody + 'static,
{
    let state = req
        .app_data::<Data<AppState>>()
        .ok_or_else(|| AppError::Internal("AppState missing".to_string()))?;

    let token = extract_bearer(&req)
        .ok_or_else(|| AppError::Unauthorized("missing bearer token".to_string()))?;

    let claims = state.tokens.verify(&token)?;

    let user = state
        .users
        .user_repository
        .find_by_id(claims.sub)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user no longer exists".to_string()))?;

    req.extensions_mut().insert(AuthUser {
        user_id: user.id,
        email: user.email.clone(),
        role: user.role()?,
    });

    next.call(req).await
}

fn extract_bearer(req: &ServiceRequest) -> Option<String> {
    let value = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}
