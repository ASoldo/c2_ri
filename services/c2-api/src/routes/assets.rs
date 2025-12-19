use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use c2_core::{Asset, SecurityClassification};
use c2_identity::Permission;
use c2_storage::AssetRepository;
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

#[get("/v1/assets")]
pub async fn list_assets(
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

    match AssetRepository::list_by_tenant(&state.store, tenant_id, limit, offset).await {
        Ok(assets) => HttpResponse::Ok().json(assets),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/assets/{id}")]
pub async fn get_asset(
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
    let asset_id = c2_core::AssetId::from_uuid(uuid);

    match AssetRepository::get(&state.store, asset_id).await {
        Ok(Some(asset)) => HttpResponse::Ok().json(asset),
        Ok(None) => not_found("asset not found"),
        Err(err) => internal_error(err.message),
    }
}

#[post("/v1/assets")]
pub async fn upsert_asset(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<Asset>,
) -> HttpResponse {
    let asset = payload.into_inner();
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::DispatchAssets,
        asset.classification,
    ) {
        return response;
    }
    if asset.name.trim().is_empty() {
        return bad_request("asset name is required");
    }

    match AssetRepository::upsert(&state.store, asset.clone()).await {
        Ok(()) => HttpResponse::Ok().json(asset),
        Err(err) => internal_error(err.message),
    }
}

#[delete("/v1/assets/{id}")]
pub async fn delete_asset(
    req: HttpRequest,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::DispatchAssets,
        SecurityClassification::Restricted,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let asset_id = c2_core::AssetId::from_uuid(uuid);

    match AssetRepository::delete(&state.store, asset_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => internal_error(err.message),
    }
}
