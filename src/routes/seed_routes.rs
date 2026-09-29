use actix_web::web;

use crate::controllers::seed_controller;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/seed").route("/users", web::post().to(seed_controller::seed_users)));
}
