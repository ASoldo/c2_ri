use c2_config::ServiceConfig;
use c2_observability::{init, log_startup, ObservabilityConfig};
use c2_storage_surreal::{SurrealConfig, SurrealStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServiceConfig::from_env("c2-worker");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
        metrics_addr: config.metrics_addr.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let surreal_config = SurrealConfig::from_env();
    let _store = SurrealStore::connect_with_retry(&surreal_config).await?;

    // TODO: connect to messaging bus and start background processing loops.
    let _data_dir = config.data_dir;
    wait_for_shutdown().await;
    Ok(())
}

async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminate =
            signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let mut interrupt =
            signal(SignalKind::interrupt()).expect("install SIGINT handler");
        tokio::select! {
            _ = terminate.recv() => {},
            _ = interrupt.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
