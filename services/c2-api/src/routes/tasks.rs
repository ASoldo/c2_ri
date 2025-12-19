use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use c2_core::{SecurityClassification, Task};
use c2_identity::Permission;
use c2_storage::TaskRepository;
use serde::Deserialize;

use crate::auth::authorize_request;
use crate::routes::common::{bad_request, internal_error, not_found, parse_uuid};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[get("/v1/missions/{mission_id}/tasks")]
pub async fn list_tasks(
    req: HttpRequest,
    state: web::Data<AppState>,
    mission_id: web::Path<String>,
    query: web::Query<ListQuery>,
) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewMissions,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }
    let uuid = match parse_uuid(&mission_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mission_id = c2_core::MissionId::from_uuid(uuid);
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    match TaskRepository::list_by_mission(&state.store, mission_id, limit, offset).await {
        Ok(tasks) => HttpResponse::Ok().json(tasks),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/tasks/{id}")]
pub async fn get_task(
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
    let task_id = c2_core::TaskId::from_uuid(uuid);

    match TaskRepository::get(&state.store, task_id).await {
        Ok(Some(task)) => HttpResponse::Ok().json(task),
        Ok(None) => not_found("task not found"),
        Err(err) => internal_error(err.message),
    }
}

#[post("/v1/tasks")]
pub async fn upsert_task(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<Task>,
) -> HttpResponse {
    let task = payload.into_inner();
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::EditMissions,
        task.classification,
    ) {
        return response;
    }
    if task.title.trim().is_empty() {
        return bad_request("task title is required");
    }

    match TaskRepository::upsert(&state.store, task.clone()).await {
        Ok(()) => HttpResponse::Ok().json(task),
        Err(err) => internal_error(err.message),
    }
}

#[delete("/v1/tasks/{id}")]
pub async fn delete_task(
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
    let task_id = c2_core::TaskId::from_uuid(uuid);

    match TaskRepository::delete(&state.store, task_id).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => internal_error(err.message),
    }
}
