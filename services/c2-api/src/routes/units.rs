use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use c2_core::{SecurityClassification, Unit};
use c2_identity::Permission;
use c2_storage::UnitRepository;
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

#[get("/v1/units")]
pub async fn list_units(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<ListQuery>,
) -> HttpResponse {
    let auth = match authorize_request(
        &req,
        &state.policy,
        Permission::ViewUnits,
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

    match UnitRepository::list_by_tenant(&state.store, tenant_id, limit, offset).await {
        Ok(units) => HttpResponse::Ok().json(units),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/units/{id}")]
pub async fn get_unit(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewUnits,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let unit_id = c2_core::UnitId::from_uuid(uuid);

    match UnitRepository::get(&state.store, unit_id).await {
        Ok(Some(unit)) => HttpResponse::Ok().json(unit),
        Ok(None) => not_found("unit not found"),
        Err(err) => internal_error(err.message),
    }
}

#[post("/v1/units")]
pub async fn upsert_unit(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<Unit>,
) -> HttpResponse {
    let unit = payload.into_inner();
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditUnits,
        unit.classification,
    ) {
        return response;
    }
    if unit.display_name.trim().is_empty() {
        return bad_request("unit display_name is required");
    }

    match UnitRepository::upsert(&state.store, unit.clone()).await {
        Ok(()) => HttpResponse::Ok().json(unit),
        Err(err) => internal_error(err.message),
    }
}

#[delete("/v1/units/{id}")]
pub async fn delete_unit(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditUnits,
        SecurityClassification::Restricted,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let unit_id = c2_core::UnitId::from_uuid(uuid);

    match UnitRepository::delete(&state.store, unit_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => internal_error(err.message),
    }
}
