use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use c2_core::{Incident, SecurityClassification};
use c2_identity::Permission;
use c2_storage::IncidentRepository;
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

#[get("/v1/incidents")]
pub async fn list_incidents(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> HttpResponse {
    let auth = match authorize_request(
        &req,
        &state.policy,
        Permission::ViewIncidents,
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

    match IncidentRepository::list_by_tenant(&state.store, tenant_id, limit, offset).await {
        Ok(incidents) => HttpResponse::Ok().json(incidents),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/incidents/{id}")]
pub async fn get_incident(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewIncidents,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let incident_id = c2_core::IncidentId::from_uuid(uuid);

    match IncidentRepository::get(&state.store, incident_id).await {
        Ok(Some(incident)) => HttpResponse::Ok().json(incident),
        Ok(None) => not_found("incident not found"),
        Err(err) => internal_error(err.message),
    }
}

#[post("/v1/incidents")]
pub async fn upsert_incident(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<Incident>,
) -> HttpResponse {
    let incident = payload.into_inner();
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::IngestData,
        incident.classification,
    ) {
        return response;
    }
    if incident.summary.trim().is_empty() {
        return bad_request("incident summary is required");
    }

    match IncidentRepository::upsert(&state.store, incident.clone()).await {
        Ok(()) => HttpResponse::Ok().json(incident),
        Err(err) => internal_error(err.message),
    }
}

#[delete("/v1/incidents/{id}")]
pub async fn delete_incident(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::IngestData,
        SecurityClassification::Restricted,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let incident_id = c2_core::IncidentId::from_uuid(uuid);

    match IncidentRepository::delete(&state.store, incident_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => internal_error(err.message),
    }
}
