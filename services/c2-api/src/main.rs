mod auth;
mod routes;
mod state;

use actix_web::{web, App, HttpServer};
use c2_config::ServiceConfig;
use c2_observability::{init, log_startup, ObservabilityConfig};
use c2_policy::BasicPolicyEngine;
use c2_storage_surreal::{SurrealConfig, SurrealStore};
use state::AppState;
use std::io;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let config = ServiceConfig::from_env("c2-api");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
        metrics_addr: config.metrics_addr.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let bind_addr = config.bind_addr.clone();
    let store = SurrealStore::connect_with_retry(&SurrealConfig::from_env())
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.message))?;
    let policy = BasicPolicyEngine::with_default_rules();
    let state = web::Data::new(AppState {
        config,
        policy,
        store,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(routes::configure)
    })
    .bind(bind_addr)?
    .run()
    .await
}
