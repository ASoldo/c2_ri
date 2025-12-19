use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub service_name: String,
    pub environment: String,
    pub log_level: String,
}

#[derive(Debug, Clone)]
pub struct ObservabilityHandle {
    pub service_name: String,
}

pub fn init(config: &ObservabilityConfig) -> ObservabilityHandle {
    let filter = EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    ObservabilityHandle {
        service_name: config.service_name.clone(),
    }
}

pub fn log_startup(handle: &ObservabilityHandle, environment: &str) {
    tracing::info!(
        service = %handle.service_name,
        environment = %environment,
        "C2 service starting"
    );
}
