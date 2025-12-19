pub mod health;
pub mod status;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health).service(status::status);
}
