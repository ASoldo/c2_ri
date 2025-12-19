use actix_web::{get, web, HttpResponse};
use c2_core::now_epoch_millis;
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Serialize)]
struct StatusResponse {
    service: String,
    environment: String,
    region: Option<String>,
    timestamp_ms: u64,
}

#[get("/v1/status")]
pub async fn status(state: web::Data<AppState>) -> HttpResponse {
    let response = StatusResponse {
        service: state.config.service_name.clone(),
        environment: state.config.environment.to_string(),
        region: state.config.region.clone(),
        timestamp_ms: now_epoch_millis(),
    };

    HttpResponse::Ok().json(response)
}
