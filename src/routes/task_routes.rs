use actix_web::{middleware::from_fn, web};

use crate::controllers::task_controller;
use crate::middleware::auth::require_auth;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tasks")
            .wrap(from_fn(require_auth))
            .route("", web::post().to(task_controller::create_task))
            .route("/assign", web::post().to(task_controller::assign_tasks))
            .route(
                "/view-my-tasks",
                web::get().to(task_controller::view_my_tasks),
            )
            .route("/{id}", web::patch().to(task_controller::patch_task)),
    );
}
