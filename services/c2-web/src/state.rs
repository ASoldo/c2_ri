use c2_config::ServiceConfig;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tera::Tera;

use crate::api::ApiClient;
use crate::flights::FlightCache;
use crate::satellites::SatelliteCache;
use crate::ships::ShipCache;
use crate::tiles::TileProvider;

pub struct AppState {
    pub config: ServiceConfig,
    pub tera: Tera,
    pub api: ApiClient,
    pub tile_config_json: Option<String>,
    pub tile_providers: HashMap<String, TileProvider>,
    pub tile_client: Client,
    pub weather_config_json: Option<String>,
    pub weather_enabled: bool,
    pub weather_base_url: String,
    pub weather_tile_matrix_set: String,
    pub weather_fields: Vec<String>,
    pub weather_default_field: String,
    pub weather_default_time: String,
    pub weather_default_format: String,
    pub weather_min_zoom: u8,
    pub weather_max_zoom: u8,
    pub sea_config_json: Option<String>,
    pub sea_enabled: bool,
    pub sea_base_url: String,
    pub sea_tile_matrix_set: String,
    pub sea_fields: Vec<String>,
    pub sea_default_field: String,
    pub sea_default_time: String,
    pub sea_default_format: String,
    pub sea_min_zoom: u8,
    pub sea_max_zoom: u8,
    pub flight_config_json: Option<String>,
    pub flight_enabled: bool,
    pub flight_provider: String,
    pub flight_base_url: String,
    pub flight_username: Option<String>,
    pub flight_password: Option<String>,
    pub flight_min_interval: Duration,
    pub flight_cache_ttl: Duration,
    pub flight_max_flights: usize,
    pub flight_sample_enabled: bool,
    pub flight_sample_count: usize,
    pub flight_cache: Mutex<FlightCache>,
    pub satellite_config_json: Option<String>,
    pub satellite_enabled: bool,
    pub satellite_provider: String,
    pub satellite_base_url: String,
    pub satellite_min_interval: Duration,
    pub satellite_cache_ttl: Duration,
    pub satellite_timeout: Duration,
    pub satellite_max: usize,
    pub satellite_sample_enabled: bool,
    pub satellite_sample_count: usize,
    pub satellite_cache: Mutex<SatelliteCache>,
    pub ship_config_json: Option<String>,
    pub ship_enabled: bool,
    pub ship_provider: String,
    pub ship_base_url: String,
    pub ship_username: Option<String>,
    pub ship_min_interval: Duration,
    pub ship_cache_ttl: Duration,
    pub ship_max_ships: usize,
    pub ship_sample_enabled: bool,
    pub ship_sample_count: usize,
    pub ship_cache: Mutex<ShipCache>,
}
