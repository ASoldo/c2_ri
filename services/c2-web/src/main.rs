mod routes;
mod state;

use actix_web::{web, App, HttpServer};
use c2_config::ServiceConfig;
use c2_observability::{init, log_startup, ObservabilityConfig};
use state::AppState;
use std::io;
use tera::Tera;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let config = ServiceConfig::from_env("c2-web");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let template_glob = format!("{}/templates/**/*", env!("CARGO_MANIFEST_DIR"));
    let tera = Tera::new(&template_glob).expect("Failed to load templates");
    let bind_addr = config.bind_addr.clone();
    let state = web::Data::new(AppState { config, tera });

    HttpServer::new(move || App::new().app_data(state.clone()).configure(routes::configure))
        .bind(bind_addr)?
        .run()
        .await
}
