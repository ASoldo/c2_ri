mod routes;

use actix_web::{web, App, HttpServer};
use c2_config::ServiceConfig;
use c2_observability::{init, log_startup, ObservabilityConfig};
use std::io;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let config = ServiceConfig::from_env("c2-api");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let bind_addr = config.bind_addr.clone();
    let shared_config = web::Data::new(config);

    HttpServer::new(move || {
        App::new()
            .app_data(shared_config.clone())
            .configure(routes::configure)
    })
    .bind(bind_addr)?
    .run()
    .await
}
