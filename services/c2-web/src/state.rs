use c2_config::ServiceConfig;
use reqwest::Client;
use std::collections::HashMap;
use tera::Tera;

use crate::api::ApiClient;
use crate::tiles::TileProvider;

pub struct AppState {
    pub config: ServiceConfig,
    pub tera: Tera,
    pub api: ApiClient,
    pub tile_config_json: Option<String>,
    pub tile_providers: HashMap<String, TileProvider>,
    pub tile_client: Client,
    pub weather_config_json: Option<String>,
    pub weather_api_key: Option<String>,
    pub weather_fields: Vec<String>,
    pub weather_default_field: String,
    pub weather_default_time: String,
    pub weather_default_format: String,
    pub weather_min_zoom: u8,
    pub weather_max_zoom: u8,
}
