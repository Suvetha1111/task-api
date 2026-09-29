use actix_web::web;

use crate::controllers::auth_controller;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(auth_controller::login))
            .route("/verify-2fa", web::post().to(auth_controller::verify_2fa)),
    );
}
