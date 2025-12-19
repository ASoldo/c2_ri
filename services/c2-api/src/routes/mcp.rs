use actix_web::{get, post, web, HttpRequest, HttpResponse};
use c2_core::{now_epoch_millis, SecurityClassification};
use c2_identity::Permission;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::authorize_request;
use crate::routes::common::{bad_request, internal_error};
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct McpCapabilities {
    version: &'static str,
    protocols: Vec<&'static str>,
    transports: Vec<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct McpHandshakeRequest {
    pub client_id: String,
    pub desired: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct McpHandshakeResponse {
    pub session_id: String,
    pub accepted: Vec<String>,
    pub issued_at_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct McpSessionRequest {
    pub client_id: String,
    pub mission_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct McpSessionResponse {
    pub session_id: String,
    pub issued_at_ms: u64,
}

#[get("/v1/mcp/capabilities")]
pub async fn capabilities(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::IngestData,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }

    HttpResponse::Ok().json(McpCapabilities {
        version: "0.1",
        protocols: vec!["json", "protobuf"],
        transports: vec!["sse", "websocket"],
    })
}

#[post("/v1/mcp/handshake")]
pub async fn handshake(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<McpHandshakeRequest>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::IngestData,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }

    let request = payload.into_inner();
    if request.client_id.trim().is_empty() {
        return bad_request("client_id is required");
    }

    let session_id = Uuid::new_v4().to_string();
    HttpResponse::Ok().json(McpHandshakeResponse {
        session_id,
        accepted: request.desired,
        issued_at_ms: now_epoch_millis(),
    })
}

#[post("/v1/mcp/sessions")]
pub async fn create_session(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<McpSessionRequest>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::IngestData,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }

    let request = payload.into_inner();
    if request.client_id.trim().is_empty() {
        return bad_request("client_id is required");
    }
    if let Some(mission_id) = request.mission_id.as_ref() {
        if Uuid::parse_str(mission_id).is_err() {
            return bad_request("invalid mission_id");
        }
    }

    let session_id = Uuid::new_v4().to_string();
    HttpResponse::Ok().json(McpSessionResponse {
        session_id,
        issued_at_ms: now_epoch_millis(),
    })
}

#[post("/v1/mcp/heartbeat")]
pub async fn heartbeat(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::IngestData,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp_ms": now_epoch_millis()
    }))
}

#[post("/v1/mcp/ingest")]
pub async fn ingest(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: String,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::IngestData,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }

    if body.trim().is_empty() {
        return bad_request("empty ingest payload");
    }

    let _ = state; // placeholder for ingest pipeline
    HttpResponse::Accepted().json(serde_json::json!({
        "status": "accepted",
        "received_bytes": body.len()
    }))
}

#[post("/v1/mcp/error")]
pub async fn error_report(
    _req: HttpRequest,
    _state: web::Data<AppState>,
) -> HttpResponse {
    internal_error("mcp error reporting not implemented")
}
