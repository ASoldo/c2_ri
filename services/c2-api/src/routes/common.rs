use actix_web::HttpResponse;
use c2_core::TenantId;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn bad_request(message: impl Into<String>) -> HttpResponse {
    HttpResponse::BadRequest().json(ErrorResponse {
        error: message.into(),
    })
}

pub fn unauthorized(message: impl Into<String>) -> HttpResponse {
    HttpResponse::Unauthorized().json(ErrorResponse {
        error: message.into(),
    })
}

pub fn forbidden(message: impl Into<String>) -> HttpResponse {
    HttpResponse::Forbidden().json(ErrorResponse {
        error: message.into(),
    })
}

pub fn not_found(message: impl Into<String>) -> HttpResponse {
    HttpResponse::NotFound().json(ErrorResponse {
        error: message.into(),
    })
}

pub fn internal_error(message: impl Into<String>) -> HttpResponse {
    HttpResponse::InternalServerError().json(ErrorResponse {
        error: message.into(),
    })
}

pub fn parse_uuid(value: &str) -> Result<Uuid, HttpResponse> {
    Uuid::parse_str(value).map_err(|_| bad_request("invalid UUID"))
}

pub fn parse_tenant_id(value: &str) -> Result<TenantId, HttpResponse> {
    let uuid = parse_uuid(value)?;
    Ok(TenantId::from_uuid(uuid))
}
