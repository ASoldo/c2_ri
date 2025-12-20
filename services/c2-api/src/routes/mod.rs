pub mod health;
pub mod capabilities;
pub mod assets;
pub mod common;
pub mod incidents;
pub mod missions;
pub mod mcp;
pub mod protobuf;
pub mod sse;
pub mod status;
pub mod teams;
pub mod tasks;
pub mod units;
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
        .service(units::list_units)
        .service(units::get_unit)
        .service(units::upsert_unit)
        .service(units::delete_unit)
        .service(teams::list_teams)
        .service(teams::get_team)
        .service(teams::upsert_team)
        .service(teams::delete_team)
        .service(capabilities::list_capabilities)
        .service(capabilities::get_capability)
        .service(capabilities::upsert_capability)
        .service(capabilities::delete_capability)
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
