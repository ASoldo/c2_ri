mod api;
mod render;
mod routes;
mod state;

use actix_files::Files;
use actix_web::{web, App, HttpServer};
use c2_config::ServiceConfig;
use c2_observability::{init, log_startup, ObservabilityConfig};
use api::ApiClient;
use state::AppState;
use std::env;
use std::io;
use std::path::Path;
use tera::Tera;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let config = ServiceConfig::from_env("c2-web");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
        metrics_addr: config.metrics_addr.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let template_root =
        env::var("C2_WEB_TEMPLATES_DIR").unwrap_or_else(|_| "templates".to_string());
    let template_glob = format!("{}/**/*", template_root);
    let tera = Tera::new(&template_glob).expect("Failed to load templates");
    let static_root = env::var("C2_WEB_STATIC_DIR").unwrap_or_else(|_| "static".to_string());
    let static_root = if Path::new(&static_root).exists() {
        static_root
    } else {
        "services/c2-web/static".to_string()
    };
    let bind_addr = config.bind_addr.clone();
    let api = ApiClient::from_env()
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.message))?;
    let tile_config_json = env::var("C2_WEB_TILE_CONFIG")
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .map(|value| value.to_string());
    let state = web::Data::new(AppState {
        config,
        tera,
        api,
        tile_config_json,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(Files::new("/static", static_root.clone()).prefer_utf8(true))
            .configure(routes::configure)
    })
        .bind(bind_addr)?
        .run()
        .await
}
