use async_trait::async_trait;
use c2_config::GatewayConfig;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora::Result;

#[derive(Debug, Clone)]
pub struct GatewayProxy {
    config: GatewayConfig,
}

impl GatewayProxy {
    pub fn new(config: GatewayConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ProxyHttp for GatewayProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let path = session.req_header().uri.path();
        let upstream = if path.starts_with("/v1") || path.starts_with("/health") {
            &self.config.api
        } else {
            &self.config.web
        };
        let sni = upstream.sni.clone().unwrap_or_else(|| upstream.host.clone());
        let peer = Box::new(HttpPeer::new(
            (upstream.host.as_str(), upstream.port),
            upstream.tls,
            sni,
        ));
        Ok(peer)
    }
}
