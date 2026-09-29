use std::sync::Arc;

use async_trait::async_trait;

use crate::error::AppError;
use crate::models::{LoginRequest, LoginResponse, TokenResponse, Verify2faRequest};
use crate::services::AuthService;

#[async_trait]
pub trait AuthManager: Send + Sync + 'static {
    async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AppError>;
    async fn verify_2fa(&self, req: Verify2faRequest) -> Result<TokenResponse, AppError>;
}

pub struct AuthManagerImpl {
    auth: Arc<dyn AuthService>,
}

impl AuthManagerImpl {
    pub fn new(auth: Arc<dyn AuthService>) -> Self {
        Self { auth }
    }
}

#[async_trait]
impl AuthManager for AuthManagerImpl {
    async fn login(&self, req: LoginRequest) -> Result<LoginResponse, AppError> {
        self.auth.login(&req.email, &req.password).await
    }

    async fn verify_2fa(&self, req: Verify2faRequest) -> Result<TokenResponse, AppError> {
        self.auth
            .verify_2fa(req.login_challenge_id, &req.code)
            .await
    }
}
