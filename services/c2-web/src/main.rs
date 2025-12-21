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
    let weather_base_url = env::var("C2_WEB_WEATHER_BASE_URL")
        .unwrap_or_else(|_| "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best".to_string());
    let weather_base_url = weather_base_url.trim_end_matches('/').to_string();
    let weather_tile_matrix_set = env::var("C2_WEB_WEATHER_TILE_MATRIX_SET")
        .unwrap_or_else(|_| "GoogleMapsCompatible_Level6".to_string());
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
                "IMERG_Precipitation_Rate".to_string(),
                "AIRS_Precipitation_Day".to_string(),
                "MODIS_Terra_Cloud_Fraction_Day".to_string(),
                "MODIS_Terra_Cloud_Top_Temp_Day".to_string(),
                "MODIS_Terra_Cloud_Top_Pressure_Day".to_string(),
                "MODIS_Terra_Cloud_Top_Height_Day".to_string(),
                "MERRA2_2m_Air_Temperature_Monthly".to_string(),
            ]
        });
    let weather_default_field = env::var("C2_WEB_WEATHER_DEFAULT_FIELD")
        .ok()
        .filter(|field| weather_fields.contains(field))
        .unwrap_or_else(|| {
            weather_fields
                .first()
                .cloned()
                .unwrap_or_else(|| "IMERG_Precipitation_Rate".to_string())
        });
    let weather_default_time =
        env::var("C2_WEB_WEATHER_DEFAULT_TIME").unwrap_or_else(|_| "default".to_string());
    let weather_default_format =
        env::var("C2_WEB_WEATHER_DEFAULT_FORMAT").unwrap_or_else(|_| "png".to_string());
    let weather_default_opacity = env::var("C2_WEB_WEATHER_DEFAULT_OPACITY")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.55);
    let weather_max_tiles = env::var("C2_WEB_WEATHER_MAX_TILES")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(24);
    let weather_update_ms = env::var("C2_WEB_WEATHER_UPDATE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(2000);
    let weather_max_in_flight = env::var("C2_WEB_WEATHER_MAX_IN_FLIGHT")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(2);
    let weather_min_zoom = env::var("C2_WEB_WEATHER_MIN_ZOOM")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    let weather_max_zoom = env::var("C2_WEB_WEATHER_MAX_ZOOM")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(6);
    let weather_enabled = env::var("C2_WEB_WEATHER_ENABLED")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value == "0" || value == "false" || value == "no" || value == "off")
        })
        .unwrap_or(true)
        && !weather_fields.is_empty();
    let weather_config_json = serde_json::json!({
        "enabled": weather_enabled,
        "fields": weather_fields.clone(),
        "defaultField": weather_default_field.clone(),
        "defaultTime": weather_default_time.clone(),
        "defaultFormat": weather_default_format.clone(),
        "defaultOpacity": weather_default_opacity,
        "maxTiles": weather_max_tiles,
        "updateIntervalMs": weather_update_ms,
        "maxInFlight": weather_max_in_flight,
        "minZoom": weather_min_zoom,
        "maxZoom": weather_max_zoom,
        "source": "NASA GIBS",
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
        weather_enabled,
        weather_base_url,
        weather_tile_matrix_set,
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
