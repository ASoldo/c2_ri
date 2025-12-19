use actix_web::{get, web, HttpRequest, HttpResponse};
use c2_core::{SecurityClassification, Task};
use c2_identity::Permission;
use c2_proto::c2 as proto;
use c2_storage::{MissionRepository, TaskRepository};
use prost::Message;

use crate::auth::authorize_request;
use crate::routes::common::{bad_request, internal_error, not_found, parse_uuid};
use crate::state::AppState;

#[get("/v1/missions/{id}/proto")]
pub async fn mission_proto(
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
        Ok(Some(mission)) => {
            let message = mission_to_proto(&mission);
            HttpResponse::Ok()
                .content_type("application/x-protobuf")
                .body(message.encode_to_vec())
        }
        Ok(None) => not_found("mission not found"),
        Err(err) => internal_error(err.message),
    }
}

#[get("/v1/tasks/{id}/proto")]
pub async fn task_proto(
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
        Ok(Some(task)) => {
            let message = task_to_proto(&task);
            HttpResponse::Ok()
                .content_type("application/x-protobuf")
                .body(message.encode_to_vec())
        }
        Ok(None) => not_found("task not found"),
        Err(err) => internal_error(err.message),
    }
}

fn mission_to_proto(mission: &c2_core::Mission) -> proto::Mission {
    proto::Mission {
        id: mission.id.to_string(),
        tenant_id: mission.tenant_id.to_string(),
        name: mission.name.clone(),
        status: map_mission_status(mission.status),
        priority: map_priority(mission.priority),
        classification: map_classification(mission.classification),
        created_at_ms: mission.created_at_ms,
        updated_at_ms: mission.updated_at_ms,
    }
}

fn task_to_proto(task: &Task) -> proto::Task {
    proto::Task {
        id: task.id.to_string(),
        mission_id: task.mission_id.to_string(),
        tenant_id: task.tenant_id.to_string(),
        title: task.title.clone(),
        status: map_task_status(task.status),
        priority: map_priority(task.priority),
        classification: map_classification(task.classification),
        created_at_ms: task.created_at_ms,
        updated_at_ms: task.updated_at_ms,
    }
}

fn map_mission_status(status: c2_core::MissionStatus) -> i32 {
    match status {
        c2_core::MissionStatus::Planned => proto::MissionStatus::Planned as i32,
        c2_core::MissionStatus::Active => proto::MissionStatus::Active as i32,
        c2_core::MissionStatus::Suspended => proto::MissionStatus::Suspended as i32,
        c2_core::MissionStatus::Completed => proto::MissionStatus::Completed as i32,
        c2_core::MissionStatus::Aborted => proto::MissionStatus::Aborted as i32,
    }
}

fn map_task_status(status: c2_core::TaskStatus) -> i32 {
    match status {
        c2_core::TaskStatus::Pending => proto::TaskStatus::Pending as i32,
        c2_core::TaskStatus::InProgress => proto::TaskStatus::InProgress as i32,
        c2_core::TaskStatus::Blocked => proto::TaskStatus::Blocked as i32,
        c2_core::TaskStatus::Completed => proto::TaskStatus::Completed as i32,
        c2_core::TaskStatus::Cancelled => proto::TaskStatus::Cancelled as i32,
    }
}

fn map_priority(priority: c2_core::OperationalPriority) -> i32 {
    match priority {
        c2_core::OperationalPriority::Routine => proto::OperationalPriority::Routine as i32,
        c2_core::OperationalPriority::Elevated => proto::OperationalPriority::Elevated as i32,
        c2_core::OperationalPriority::Urgent => proto::OperationalPriority::Urgent as i32,
        c2_core::OperationalPriority::Critical => proto::OperationalPriority::Critical as i32,
    }
}

fn map_classification(classification: c2_core::SecurityClassification) -> i32 {
    match classification {
        c2_core::SecurityClassification::Unclassified => {
            proto::SecurityClassification::Unclassified as i32
        }
        c2_core::SecurityClassification::Controlled => {
            proto::SecurityClassification::Controlled as i32
        }
        c2_core::SecurityClassification::Restricted => {
            proto::SecurityClassification::Restricted as i32
        }
        c2_core::SecurityClassification::Confidential => {
            proto::SecurityClassification::Confidential as i32
        }
        c2_core::SecurityClassification::Secret => proto::SecurityClassification::Secret as i32,
        c2_core::SecurityClassification::TopSecret => {
            proto::SecurityClassification::TopSecret as i32
        }
    }
}

#[allow(dead_code)]
fn parse_proto_status(value: i32) -> Result<proto::MissionStatus, HttpResponse> {
    proto::MissionStatus::try_from(value)
        .map_err(|_| bad_request("invalid mission status"))
}
