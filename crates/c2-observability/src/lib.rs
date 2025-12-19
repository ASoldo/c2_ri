use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub service_name: String,
    pub environment: String,
    pub log_level: String,
    pub metrics_addr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ObservabilityHandle {
    pub service_name: String,
    pub metrics_enabled: bool,
}

pub fn init(config: &ObservabilityConfig) -> ObservabilityHandle {
    let filter = EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    let metrics_enabled = init_metrics(config);

    ObservabilityHandle {
        service_name: config.service_name.clone(),
        metrics_enabled,
    }
}

pub fn log_startup(handle: &ObservabilityHandle, environment: &str) {
    tracing::info!(
        service = %handle.service_name,
        environment = %environment,
        metrics_enabled = handle.metrics_enabled,
        "C2 service starting"
    );
}

fn init_metrics(config: &ObservabilityConfig) -> bool {
    let Some(addr) = config.metrics_addr.as_ref() else {
        return false;
    };
    let addr: SocketAddr = match addr.parse() {
        Ok(parsed) => parsed,
        Err(err) => {
            tracing::warn!(
                service = %config.service_name,
                error = %err,
                "Invalid C2_METRICS_ADDR value"
            );
            return false;
        }
    };

    let builder = PrometheusBuilder::new()
        .with_http_listener(addr)
        .add_global_label("service", config.service_name.clone())
        .add_global_label("environment", config.environment.clone());

    match builder.install() {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(
                service = %config.service_name,
                error = %err,
                "Failed to initialize Prometheus exporter"
            );
            false
        }
    }
}
