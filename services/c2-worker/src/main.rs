use c2_config::ServiceConfig;
use c2_observability::{init, log_startup, ObservabilityConfig};

fn main() {
    let config = ServiceConfig::from_env("c2-worker");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    // TODO: connect to messaging bus and start background processing loops.
    let _data_dir = config.data_dir;
}
