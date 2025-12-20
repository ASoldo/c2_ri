use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use c2_core::{SecurityClassification, Team};
use c2_identity::Permission;
use c2_storage::TeamRepository;
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

#[get("/v1/teams")]
pub async fn list_teams(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> HttpResponse {
    let auth = match authorize_request(
        &req,
        &state.policy,
        Permission::ViewTeams,
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

    match TeamRepository::list_by_tenant(&state.store, tenant_id, limit, offset).await {
        Ok(teams) => HttpResponse::Ok().json(teams),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/teams/{id}")]
pub async fn get_team(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewTeams,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let team_id = c2_core::TeamId::from_uuid(uuid);

    match TeamRepository::get(&state.store, team_id).await {
        Ok(Some(team)) => HttpResponse::Ok().json(team),
        Ok(None) => not_found("team not found"),
        Err(err) => internal_error(err.message),
    }
}

#[post("/v1/teams")]
pub async fn upsert_team(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<Team>,
) -> HttpResponse {
    let team = payload.into_inner();
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditTeams,
        team.classification,
    ) {
        return response;
    }
    if team.name.trim().is_empty() {
        return bad_request("team name is required");
    }

    match TeamRepository::upsert(&state.store, team.clone()).await {
        Ok(()) => HttpResponse::Ok().json(team),
        Err(err) => internal_error(err.message),
    }
}

#[delete("/v1/teams/{id}")]
pub async fn delete_team(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditTeams,
        SecurityClassification::Restricted,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let team_id = c2_core::TeamId::from_uuid(uuid);

    match TeamRepository::delete(&state.store, team_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => internal_error(err.message),
    }
}
