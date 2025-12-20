use c2_config::ServiceConfig;
use tera::Tera;

use crate::api::ApiClient;

pub struct AppState {
    pub config: ServiceConfig,
    pub tera: Tera,
    pub api: ApiClient,
}
