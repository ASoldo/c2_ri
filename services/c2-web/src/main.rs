mod api;
mod flights;
mod satellites;
mod ships;
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
use std::time::Duration;
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
    let flight_enabled = env::var("C2_WEB_FLIGHT_ENABLED")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value == "0" || value == "false" || value == "no" || value == "off")
        })
        .unwrap_or(true);
    let flight_provider =
        env::var("C2_WEB_FLIGHT_PROVIDER").unwrap_or_else(|_| "adsb_lol".to_string());
    let flight_provider_key = flight_provider.trim().to_ascii_lowercase();
    let flight_base_url = env::var("C2_WEB_FLIGHT_BASE_URL").unwrap_or_else(|_| {
        if flight_provider_key.contains("adsb") {
            "https://api.adsb.lol/v2/lat/{lat}/lon/{lon}/dist/{dist}".to_string()
        } else {
            "https://opensky-network.org/api/states/all".to_string()
        }
    });
    let flight_username = env::var("C2_WEB_FLIGHT_USER").ok();
    let flight_password = env::var("C2_WEB_FLIGHT_PASS").ok();
    let flight_update_ms = env::var("C2_WEB_FLIGHT_UPDATE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5000);
    let flight_min_interval_ms = env::var("C2_WEB_FLIGHT_MIN_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(3500);
    let flight_cache_ttl_ms = env::var("C2_WEB_FLIGHT_CACHE_TTL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(6000);
    let flight_max_flights = env::var("C2_WEB_FLIGHT_MAX")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(80);
    let flight_trail_points = env::var("C2_WEB_FLIGHT_TRAIL_POINTS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(24);
    let flight_trail_max_age_ms = env::var("C2_WEB_FLIGHT_TRAIL_MAX_AGE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(240_000);
    let flight_span_min_deg = env::var("C2_WEB_FLIGHT_SPAN_MIN_DEG")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(8.0);
    let flight_span_max_deg = env::var("C2_WEB_FLIGHT_SPAN_MAX_DEG")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(60.0);
    let flight_altitude_scale = env::var("C2_WEB_FLIGHT_ALTITUDE_SCALE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.08);
    let flight_sample_enabled = env::var("C2_WEB_FLIGHT_SAMPLE_ENABLED")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value == "0" || value == "false" || value == "no" || value == "off")
        })
        .unwrap_or(true);
    let flight_sample_count = env::var("C2_WEB_FLIGHT_SAMPLE_COUNT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(5);
    let flight_source_label = if flight_provider_key.contains("adsb") {
        "ADSB.lol".to_string()
    } else if flight_provider_key == "opensky" {
        "OpenSky".to_string()
    } else {
        flight_provider.clone()
    };
    let flight_config_json = serde_json::json!({
        "enabled": flight_enabled,
        "provider": flight_provider.clone(),
        "updateIntervalMs": flight_update_ms,
        "minIntervalMs": flight_min_interval_ms,
        "maxFlights": flight_max_flights,
        "trailPoints": flight_trail_points,
        "trailMaxAgeMs": flight_trail_max_age_ms,
        "spanMinDeg": flight_span_min_deg,
        "spanMaxDeg": flight_span_max_deg,
        "altitudeScale": flight_altitude_scale,
        "source": flight_source_label,
        "sample": flight_sample_enabled,
    })
    .to_string();
    let satellite_enabled = env::var("C2_WEB_SAT_ENABLED")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value == "0" || value == "false" || value == "no" || value == "off")
        })
        .unwrap_or(true);
    let satellite_provider =
        env::var("C2_WEB_SAT_PROVIDER").unwrap_or_else(|_| "celestrak".to_string());
    let satellite_base_url = env::var("C2_WEB_SAT_BASE_URL").unwrap_or_else(|_| {
        "https://celestrak.org/NORAD/elements/gp.php?GROUP=visual&FORMAT=json".to_string()
    });
    let satellite_source_label =
        env::var("C2_WEB_SAT_SOURCE").unwrap_or_else(|_| "CelesTrak".to_string());
    let satellite_update_ms = env::var("C2_WEB_SAT_UPDATE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(8000);
    let satellite_min_interval_ms = env::var("C2_WEB_SAT_MIN_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(6000);
    let satellite_cache_ttl_ms = env::var("C2_WEB_SAT_CACHE_TTL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30000);
    let satellite_timeout_ms = env::var("C2_WEB_SAT_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(4000);
    let satellite_max = env::var("C2_WEB_SAT_MAX")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(120);
    let satellite_altitude_scale = env::var("C2_WEB_SAT_ALTITUDE_SCALE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.018);
    let satellite_altitude_min = env::var("C2_WEB_SAT_ALTITUDE_MIN")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(4.0);
    let satellite_altitude_max = env::var("C2_WEB_SAT_ALTITUDE_MAX")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(90.0);
    let satellite_sample_enabled = env::var("C2_WEB_SAT_SAMPLE_ENABLED")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value == "0" || value == "false" || value == "no" || value == "off")
        })
        .unwrap_or(true);
    let satellite_sample_count = env::var("C2_WEB_SAT_SAMPLE_COUNT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(6);
    let satellite_config_json = serde_json::json!({
        "enabled": satellite_enabled,
        "provider": satellite_provider.clone(),
        "updateIntervalMs": satellite_update_ms,
        "maxSatellites": satellite_max,
        "altitudeScale": satellite_altitude_scale,
        "altitudeMin": satellite_altitude_min,
        "altitudeMax": satellite_altitude_max,
        "source": satellite_source_label,
        "sample": satellite_sample_enabled,
    })
    .to_string();
    let ship_enabled = env::var("C2_WEB_SHIP_ENABLED")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value == "0" || value == "false" || value == "no" || value == "off")
        })
        .unwrap_or(true);
    let ship_provider =
        env::var("C2_WEB_SHIP_PROVIDER").unwrap_or_else(|_| "arcgis".to_string());
    let ship_base_url = env::var("C2_WEB_SHIP_BASE_URL").unwrap_or_else(|_| {
        "https://services.arcgis.com/hRUr1F8lE8Jq2uJo/arcgis/rest/services/ShipPositions/FeatureServer/0/query".to_string()
    });
    let ship_source_label =
        env::var("C2_WEB_SHIP_SOURCE").unwrap_or_else(|_| "ArcGIS ShipPositions".to_string());
    let ship_update_ms = env::var("C2_WEB_SHIP_UPDATE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(9000);
    let ship_min_interval_ms = env::var("C2_WEB_SHIP_MIN_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(7000);
    let ship_cache_ttl_ms = env::var("C2_WEB_SHIP_CACHE_TTL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(45000);
    let ship_max = env::var("C2_WEB_SHIP_MAX")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(200);
    let ship_span_min_deg = env::var("C2_WEB_SHIP_SPAN_MIN_DEG")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(6.0);
    let ship_span_max_deg = env::var("C2_WEB_SHIP_SPAN_MAX_DEG")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(70.0);
    let ship_altitude = env::var("C2_WEB_SHIP_ALTITUDE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.12);
    let ship_sample_enabled = env::var("C2_WEB_SHIP_SAMPLE_ENABLED")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value == "0" || value == "false" || value == "no" || value == "off")
        })
        .unwrap_or(true);
    let ship_sample_count = env::var("C2_WEB_SHIP_SAMPLE_COUNT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(8);
    let ship_config_json = serde_json::json!({
        "enabled": ship_enabled,
        "provider": ship_provider.clone(),
        "updateIntervalMs": ship_update_ms,
        "maxShips": ship_max,
        "spanMinDeg": ship_span_min_deg,
        "spanMaxDeg": ship_span_max_deg,
        "altitude": ship_altitude,
        "source": ship_source_label,
        "sample": ship_sample_enabled,
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
        flight_config_json: Some(flight_config_json),
        flight_enabled,
        flight_provider,
        flight_base_url,
        flight_username,
        flight_password,
        flight_min_interval: Duration::from_millis(flight_min_interval_ms),
        flight_cache_ttl: Duration::from_millis(flight_cache_ttl_ms),
        flight_max_flights: flight_max_flights.max(1),
        flight_sample_enabled,
        flight_sample_count: flight_sample_count.max(1),
        flight_cache: std::sync::Mutex::new(flights::FlightCache::new()),
        satellite_config_json: Some(satellite_config_json),
        satellite_enabled,
        satellite_provider,
        satellite_base_url,
        satellite_min_interval: Duration::from_millis(satellite_min_interval_ms),
        satellite_cache_ttl: Duration::from_millis(satellite_cache_ttl_ms),
        satellite_timeout: Duration::from_millis(satellite_timeout_ms),
        satellite_max: satellite_max.max(1),
        satellite_sample_enabled,
        satellite_sample_count: satellite_sample_count.max(1),
        satellite_cache: std::sync::Mutex::new(satellites::SatelliteCache::new()),
        ship_config_json: Some(ship_config_json),
        ship_enabled,
        ship_provider,
        ship_base_url,
        ship_min_interval: Duration::from_millis(ship_min_interval_ms),
        ship_cache_ttl: Duration::from_millis(ship_cache_ttl_ms),
        ship_max_ships: ship_max.max(1),
        ship_sample_enabled,
        ship_sample_count: ship_sample_count.max(1),
        ship_cache: std::sync::Mutex::new(ships::ShipCache::new()),
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
