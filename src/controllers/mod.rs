//! Controllers parse HTTP, read the authenticated user, call one manager and
//! let `AppError`'s `ResponseError` impl map failures to status codes.

pub mod auth_controller;
pub mod dev_controller;
pub mod seed_controller;
pub mod task_controller;

use actix_web::{HttpMessage, HttpRequest};

use crate::error::AppError;
use crate::models::AuthUser;

/// The user attached by [`crate::middleware::auth::require_auth`].
pub fn auth_user(req: &HttpRequest) -> Result<AuthUser, AppError> {
    req.extensions()
        .get::<AuthUser>()
        .cloned()
        .ok_or_else(|| AppError::Unauthorized("authentication required".to_string()))
}
