//! URL -> handler mapping. Public routes are registered directly; protected
//! routes live under a scope wrapped with the auth middleware.

pub mod auth_routes;
pub mod dev_routes;
pub mod seed_routes;
pub mod task_routes;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    seed_routes::configure(cfg);
    auth_routes::configure(cfg);
    dev_routes::configure(cfg);
    task_routes::configure(cfg);
}
