use tera::Context;

use crate::api::{StatusResponse, UiSnapshot};
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct UiTemplateData {
    pub service_name: String,
    pub environment: String,
    pub status: Option<StatusResponse>,
    pub snapshot: UiSnapshot,
}

impl UiTemplateData {
    pub fn from_state(state: &AppState, status: Option<StatusResponse>, snapshot: UiSnapshot) -> Self {
        Self {
            service_name: state.config.service_name.clone(),
            environment: state.config.environment.to_string(),
            status,
            snapshot,
        }
    }
}

pub fn build_context(data: &UiTemplateData) -> Context {
    let mut context = Context::new();
    context.insert("service_name", &data.service_name);
    context.insert("environment", &data.environment);
    context.insert("status", &data.status);
    context.insert("snapshot", &data.snapshot);
    context
}
