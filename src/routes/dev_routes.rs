use actix_web::web;

use crate::controllers::dev_controller;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/dev").route(
        "/email-logs/latest",
        web::get().to(dev_controller::latest_email_log),
    ));
}
