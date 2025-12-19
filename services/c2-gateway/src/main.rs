mod proxy;

use c2_config::{GatewayConfig, ServiceConfig};
use c2_observability::{init, log_startup, ObservabilityConfig};
use pingora::proxy::http_proxy_service;
use pingora::server::Server;
use proxy::GatewayProxy;

fn main() {
    let config = ServiceConfig::from_env("c2-gateway");
    let gateway_config = GatewayConfig::from_env();
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let bind_addr = config.bind_addr.clone();
    let _trusted_proxies = config.trusted_proxies;

    let mut server = Server::new(None).expect("failed to create Pingora server");
    server.bootstrap();

    let mut proxy = http_proxy_service(&server.configuration, GatewayProxy::new(gateway_config));
    proxy.add_tcp(&bind_addr);
    server.add_service(proxy);
    server.run_forever();
}
