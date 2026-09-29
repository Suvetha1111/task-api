//! Application assembly shared by `main.rs` and the integration tests.

use std::sync::Arc;

use actix_web::web;
use deadpool_redis::{Config as RedisConfig, Runtime};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;
use crate::containers::{AuthContainer, TaskContainer, UserContainer};
use crate::error::AppError;
use crate::routes;
use crate::services::TokenServiceImpl;
use crate::state::AppState;
use crate::swagger::ApiDoc;

/// Connect to Postgres and Redis and wire every container.
pub async fn build_state(config: Config) -> Result<AppState, AppError> {
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    let redis = RedisConfig::from_url(&config.redis_url).create_pool(Some(Runtime::Tokio1))?;

    let tokens = Arc::new(TokenServiceImpl::new(
        &config.jwt_secret,
        config.jwt_expiry_secs,
    ));

    Ok(AppState {
        auth: AuthContainer::new(&pool, &config, tokens.clone())?,
        tasks: TaskContainer::new(&pool, &redis),
        users: UserContainer::new(&pool),
        tokens,
        config: Arc::new(config),
    })
}

/// Register state, Swagger UI and all routes on an actix `App`.
pub fn configure(state: AppState) -> impl FnOnce(&mut web::ServiceConfig) + Clone {
    move |cfg: &mut web::ServiceConfig| {
        cfg.app_data(web::Data::new(state))
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .configure(routes::configure);
    }
}
