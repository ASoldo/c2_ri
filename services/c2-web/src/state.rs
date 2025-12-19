use c2_config::ServiceConfig;
use tera::Tera;

pub struct AppState {
    pub config: ServiceConfig,
    pub tera: Tera,
}
