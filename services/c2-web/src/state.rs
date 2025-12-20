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
}
