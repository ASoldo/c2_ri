use c2_config::ServiceConfig;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tera::Tera;

use crate::api::ApiClient;
use crate::flights::FlightCache;
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
}
