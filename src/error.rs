use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

/// The single application error type. Every layer returns `Result<T, AppError>`.
///
/// | Variant      | HTTP |
/// |--------------|------|
/// | BadRequest   | 400  |
/// | Unauthorized | 401  |
/// | Forbidden    | 403  |
/// | NotFound     | 404  |
/// | Database     | 500  |
/// | Redis        | 500  |
/// | Internal     | 500  |
#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("cache error: {0}")]
    Redis(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    message: String,
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "bad_request",
            AppError::Unauthorized(_) => "unauthorized",
            AppError::Forbidden(_) => "forbidden",
            AppError::NotFound(_) => "not_found",
            AppError::Database(_) => "database_error",
            AppError::Redis(_) => "cache_error",
            AppError::Internal(_) => "internal_error",
        }
    }

    /// Message that is safe to show to the client. 5xx details stay in the logs.
    fn public_message(&self) -> String {
        match self {
            AppError::BadRequest(m)
            | AppError::Unauthorized(m)
            | AppError::Forbidden(m)
            | AppError::NotFound(m) => m.clone(),
            AppError::Database(_) => "database error".to_string(),
            AppError::Redis(_) => "cache unavailable".to_string(),
            AppError::Internal(_) => "internal server error".to_string(),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Database(_) | AppError::Redis(_) | AppError::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        if self.status_code().is_server_error() {
            tracing::error!(error = %self, "request failed");
        }
        HttpResponse::build(self.status_code()).json(ErrorBody {
            error: self.kind().to_string(),
            message: self.public_message(),
        })
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Redis(err.to_string())
    }
}

impl From<deadpool_redis::PoolError> for AppError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        AppError::Redis(err.to_string())
    }
}

impl From<deadpool_redis::CreatePoolError> for AppError {
    fn from(err: deadpool_redis::CreatePoolError) -> Self {
        AppError::Redis(err.to_string())
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        AppError::Internal(format!("bcrypt: {err}"))
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::Internal(format!("jwt: {err}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Internal(format!("json: {err}"))
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::BadRequest(err.to_string())
    }
}
