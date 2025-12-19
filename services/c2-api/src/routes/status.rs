use actix_web::{get, web, HttpResponse};
use c2_config::ServiceConfig;
use c2_core::now_epoch_millis;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct StatusResponse {
    service: String,
    environment: String,
    region: Option<String>,
    timestamp_ms: u64,
}

#[get("/v1/status")]
pub async fn status(config: web::Data<ServiceConfig>) -> HttpResponse {
    let response = StatusResponse {
        service: config.service_name.clone(),
        environment: config.environment.to_string(),
        region: config.region.clone(),
        timestamp_ms: now_epoch_millis(),
    };

    HttpResponse::Ok().json(response)
}
