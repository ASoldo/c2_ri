pub mod health;
pub mod assets;
pub mod common;
pub mod incidents;
pub mod missions;
pub mod mcp;
pub mod protobuf;
pub mod sse;
pub mod status;
pub mod tasks;
pub mod ws;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health)
        .service(status::status)
        .service(missions::list_missions)
        .service(missions::get_mission)
        .service(missions::upsert_mission)
        .service(missions::delete_mission)
        .service(assets::list_assets)
        .service(assets::get_asset)
        .service(assets::upsert_asset)
        .service(assets::delete_asset)
        .service(incidents::list_incidents)
        .service(incidents::get_incident)
        .service(incidents::upsert_incident)
        .service(incidents::delete_incident)
        .service(tasks::list_tasks)
        .service(tasks::get_task)
        .service(tasks::upsert_task)
        .service(tasks::delete_task)
        .service(protobuf::mission_proto)
        .service(protobuf::task_proto)
        .service(sse::sse)
        .service(ws::ws_route)
        .service(mcp::capabilities)
        .service(mcp::handshake)
        .service(mcp::create_session)
        .service(mcp::heartbeat)
        .service(mcp::ingest)
        .service(mcp::error_report);
}
