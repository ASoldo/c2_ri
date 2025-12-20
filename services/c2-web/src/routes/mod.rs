pub mod health;
pub mod partials;
pub mod ui_api;
pub mod ui;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health)
        .service(ui::index)
        .service(ui_api::status)
        .service(ui_api::summary)
        .service(ui_api::sse)
        .service(ui_api::ws_route)
        .service(partials::mission_feed)
        .service(partials::incidents)
        .service(partials::assets);
}
