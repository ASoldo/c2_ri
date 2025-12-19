use async_trait::async_trait;
use c2_config::GatewayConfig;
use http::header::{HeaderName, AUTHORIZATION};
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

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        let Some(token) = self.config.auth.api_token.as_ref() else {
            return Ok(false);
        };

        let path = session.req_header().uri.path();
        if !path.starts_with("/v1") || self.config.auth.is_bypassed(path) {
            return Ok(false);
        }

        let header_name = self.config.auth.header_name.to_ascii_lowercase();
        let header_name = if header_name == "authorization" {
            Some(AUTHORIZATION)
        } else {
            HeaderName::from_lowercase(header_name.as_bytes()).ok()
        };

        let header_value = header_name.and_then(|name| {
            session
                .req_header()
                .headers
                .get(name)
                .and_then(|value| value.to_str().ok())
        });

        let expected = format!("Bearer {}", token);
        let authorized = header_value
            .map(|value| value == expected || value == token)
            .unwrap_or(false);

        if !authorized {
            let _ = session.respond_error(401).await;
            return Ok(true);
        }

        Ok(false)
    }

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
