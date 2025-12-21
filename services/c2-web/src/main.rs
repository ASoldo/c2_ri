mod api;
mod render;
mod routes;
mod state;
mod tiles;

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
    let tile_config_value = env::var("C2_WEB_TILE_CONFIG")
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let tile_config_json = tile_config_value.as_ref().map(|value| value.to_string());
    let tile_providers = tile_config_value
        .as_ref()
        .and_then(tiles::tile_providers_from_value)
        .unwrap_or_else(tiles::default_tile_providers);
    let tile_user_agent = env::var("C2_WEB_TILE_USER_AGENT")
        .unwrap_or_else(|_| format!("C2-Walaris/{}", env!("CARGO_PKG_VERSION")));
    let tile_client = reqwest::Client::builder()
        .user_agent(tile_user_agent)
        .build()
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
    let weather_api_key = env::var("C2_WEB_WEATHER_API_KEY").ok();
    let weather_fields = env::var("C2_WEB_WEATHER_FIELDS")
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(|field| field.trim().to_string())
                .filter(|field| !field.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|fields| !fields.is_empty())
        .unwrap_or_else(|| {
            vec![
                "cloudCover".to_string(),
                "precipitationIntensity".to_string(),
                "temperature".to_string(),
                "windSpeed".to_string(),
                "pressureSeaLevel".to_string(),
                "humidity".to_string(),
                "visibility".to_string(),
            ]
        });
    let weather_default_field = env::var("C2_WEB_WEATHER_DEFAULT_FIELD")
        .ok()
        .filter(|field| weather_fields.contains(field))
        .unwrap_or_else(|| weather_fields.first().cloned().unwrap_or_else(|| "cloudCover".to_string()));
    let weather_default_time =
        env::var("C2_WEB_WEATHER_DEFAULT_TIME").unwrap_or_else(|_| "now".to_string());
    let weather_default_format =
        env::var("C2_WEB_WEATHER_DEFAULT_FORMAT").unwrap_or_else(|_| "png".to_string());
    let weather_default_opacity = env::var("C2_WEB_WEATHER_DEFAULT_OPACITY")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.55);
    let weather_min_zoom = env::var("C2_WEB_WEATHER_MIN_ZOOM")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(1);
    let weather_max_zoom = env::var("C2_WEB_WEATHER_MAX_ZOOM")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(12);
    let weather_enabled = weather_api_key.is_some();
    let weather_config_json = serde_json::json!({
        "enabled": weather_enabled,
        "fields": weather_fields.clone(),
        "defaultField": weather_default_field.clone(),
        "defaultTime": weather_default_time.clone(),
        "defaultFormat": weather_default_format.clone(),
        "defaultOpacity": weather_default_opacity,
        "minZoom": weather_min_zoom,
        "maxZoom": weather_max_zoom,
    })
    .to_string();
    let state = web::Data::new(AppState {
        config,
        tera,
        api,
        tile_config_json,
        tile_providers,
        tile_client,
        weather_config_json: Some(weather_config_json),
        weather_api_key,
        weather_fields,
        weather_default_field,
        weather_default_time,
        weather_default_format,
        weather_min_zoom,
        weather_max_zoom,
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
