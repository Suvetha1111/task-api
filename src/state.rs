use std::sync::Arc;

use crate::config::Config;
use crate::containers::{AuthContainer, TaskContainer, UserContainer};
use crate::services::TokenService;

/// Everything a request handler needs, built once at startup and cloned into
/// every actix worker. All fields are `Arc`s or cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub tokens: Arc<dyn TokenService>,
    pub auth: AuthContainer,
    pub tasks: TaskContainer,
    pub users: UserContainer,
}
