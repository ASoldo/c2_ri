use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use c2_core::{Capability, SecurityClassification};
use c2_identity::Permission;
use c2_storage::CapabilityRepository;
use serde::Deserialize;

use crate::auth::authorize_request;
use crate::routes::common::{bad_request, internal_error, not_found, parse_tenant_id, parse_uuid};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub tenant_id: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[get("/v1/capabilities")]
pub async fn list_capabilities(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> HttpResponse {
    let auth = match authorize_request(
        &req,
        &state.policy,
        Permission::ViewCapabilities,
        SecurityClassification::Unclassified,
    ) {
        Ok(auth) => auth,
        Err(response) => return response,
    };
    let tenant_id = match parse_tenant_id(&query.tenant_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if auth.subject.tenant_id != tenant_id {
        return bad_request("tenant mismatch");
    }
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    match CapabilityRepository::list_by_tenant(&state.store, tenant_id, limit, offset).await {
        Ok(capabilities) => HttpResponse::Ok().json(capabilities),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/capabilities/{id}")]
pub async fn get_capability(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewCapabilities,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let capability_id = c2_core::CapabilityId::from_uuid(uuid);

    match CapabilityRepository::get(&state.store, capability_id).await {
        Ok(Some(capability)) => HttpResponse::Ok().json(capability),
        Ok(None) => not_found("capability not found"),
        Err(err) => internal_error(err.message),
    }
}

#[post("/v1/capabilities")]
pub async fn upsert_capability(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<Capability>,
) -> HttpResponse {
    let capability = payload.into_inner();
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditCapabilities,
        capability.classification,
    ) {
        return response;
    }
    if capability.code.trim().is_empty() {
        return bad_request("capability code is required");
    }
    if capability.name.trim().is_empty() {
        return bad_request("capability name is required");
    }

    match CapabilityRepository::upsert(&state.store, capability.clone()).await {
        Ok(()) => HttpResponse::Ok().json(capability),
        Err(err) => internal_error(err.message),
    }
}

#[delete("/v1/capabilities/{id}")]
pub async fn delete_capability(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditCapabilities,
        SecurityClassification::Restricted,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let capability_id = c2_core::CapabilityId::from_uuid(uuid);

    match CapabilityRepository::delete(&state.store, capability_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => internal_error(err.message),
    }
}
