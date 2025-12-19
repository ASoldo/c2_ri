use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use c2_core::{Mission, SecurityClassification};
use c2_identity::Permission;
use c2_storage::MissionRepository;
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

#[get("/v1/missions")]
pub async fn list_missions(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> HttpResponse {
    let auth = match authorize_request(
        &req,
        &state.policy,
        Permission::ViewMissions,
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

    match MissionRepository::list_by_tenant(&state.store, tenant_id, limit, offset).await {
        Ok(missions) => HttpResponse::Ok().json(missions),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/missions/{id}")]
pub async fn get_mission(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewMissions,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mission_id = c2_core::MissionId::from_uuid(uuid);

    match MissionRepository::get(&state.store, mission_id).await {
        Ok(Some(mission)) => HttpResponse::Ok().json(mission),
        Ok(None) => not_found("mission not found"),
        Err(err) => internal_error(err.message),
    }
}

#[post("/v1/missions")]
pub async fn upsert_mission(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<Mission>,
) -> HttpResponse {
    let mission = payload.into_inner();
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditMissions,
        mission.classification,
    ) {
        return response;
    }
    if mission.name.trim().is_empty() {
        return bad_request("mission name is required");
    }

    match MissionRepository::upsert(&state.store, mission.clone()).await {
        Ok(()) => HttpResponse::Ok().json(mission),
        Err(err) => internal_error(err.message),
    }
}

#[delete("/v1/missions/{id}")]
pub async fn delete_mission(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditMissions,
        SecurityClassification::Restricted,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mission_id = c2_core::MissionId::from_uuid(uuid);

    match MissionRepository::delete(&state.store, mission_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => internal_error(err.message),
    }
}
