pub mod health;
pub mod partials;
pub mod tiles;
pub mod ui_api;
pub mod ui;
pub mod flights;
pub mod satellites;
pub mod ships;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health)
        .service(ui::index)
        .service(ui_api::status)
        .service(ui_api::summary)
        .service(ui_api::entities)
        .service(ui_api::sse)
        .service(ui_api::ws_route)
        .service(flights::flights)
        .service(satellites::satellites)
        .service(ships::ships)
        .service(tiles::weather_tile)
        .service(tiles::tile)
        .service(partials::mission_feed)
        .service(partials::incidents)
        .service(partials::assets);
}
