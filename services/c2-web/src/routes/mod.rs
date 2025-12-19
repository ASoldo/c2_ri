pub mod health;
pub mod ui;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health).service(ui::index);
}
