use actix_web::{middleware::Logger, App, HttpServer};
use tracing_subscriber::EnvFilter;

use task_api::app::{build_state, configure};
use task_api::config::Config;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env().map_err(to_io)?;
    let host = config.server_host.clone();
    let port = config.server_port;

    let state = build_state(config).await.map_err(to_io)?;
    tracing::info!("listening on http://{host}:{port} (swagger at /swagger-ui/)");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .configure(configure(state.clone()))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

fn to_io(err: task_api::error::AppError) -> std::io::Error {
    std::io::Error::other(err.to_string())
}
