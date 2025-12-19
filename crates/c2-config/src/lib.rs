use serde::{Deserialize, Serialize};
use std::{env, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Local,
    Dev,
    Test,
    Staging,
    Prod,
}

impl Environment {
    pub fn from_env(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            "dev" | "development" => Self::Dev,
            "test" | "testing" => Self::Test,
            "staging" => Self::Staging,
            "prod" | "production" => Self::Prod,
            _ => Self::Local,
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Local => "local",
            Self::Dev => "dev",
            Self::Test => "test",
            Self::Staging => "staging",
            Self::Prod => "prod",
        };
        write!(f, "{}", value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub environment: Environment,
    pub region: Option<String>,
    pub bind_addr: String,
    pub metrics_addr: Option<String>,
    pub log_level: String,
    pub data_dir: String,
    pub trusted_proxies: Vec<String>,
}

impl ServiceConfig {
    pub fn from_env(default_service_name: &str) -> Self {
        let service_name = env_var("C2_SERVICE_NAME", default_service_name.to_string());
        let environment = Environment::from_env(&env_var("C2_ENV", "local".to_string()));
        let region = env::var("C2_REGION").ok();
        let bind_addr = env_var("C2_BIND_ADDR", "0.0.0.0:8080".to_string());
        let metrics_addr = env::var("C2_METRICS_ADDR").ok();
        let log_level = env_var("C2_LOG_LEVEL", "info".to_string());
        let data_dir = env_var("C2_DATA_DIR", "/var/lib/c2".to_string());
        let trusted_proxies = env::var("C2_TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();

        Self {
            service_name,
            environment,
            region,
            bind_addr,
            metrics_addr,
            log_level,
            data_dir,
            trusted_proxies,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayUpstream {
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub sni: Option<String>,
}

impl GatewayUpstream {
    pub fn from_env(prefix: &str, default_host: &str, default_port: u16) -> Self {
        let host_key = format!("{prefix}_HOST");
        let port_key = format!("{prefix}_PORT");
        let tls_key = format!("{prefix}_TLS");
        let sni_key = format!("{prefix}_SNI");

        Self {
            host: env_var(&host_key, default_host.to_string()),
            port: env_var_u16(&port_key, default_port),
            tls: env_var_bool(&tls_key, false),
            sni: env::var(&sni_key).ok(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayTlsConfig {
    pub bind_addr: String,
    pub cert_path: String,
    pub key_path: String,
}

impl GatewayTlsConfig {
    pub fn from_env() -> Option<Self> {
        let bind_addr = env::var("C2_GATEWAY_TLS_ADDR").ok();
        let cert_path = env::var("C2_GATEWAY_TLS_CERT").ok();
        let key_path = env::var("C2_GATEWAY_TLS_KEY").ok();

        match (bind_addr, cert_path, key_path) {
            (Some(bind_addr), Some(cert_path), Some(key_path)) => Some(Self {
                bind_addr,
                cert_path,
                key_path,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayAuthConfig {
    pub api_token: Option<String>,
    pub header_name: String,
    pub bypass_paths: Vec<String>,
}

impl GatewayAuthConfig {
    pub fn from_env() -> Self {
        let api_token = env::var("C2_GATEWAY_API_TOKEN").ok();
        let header_name = env_var("C2_GATEWAY_AUTH_HEADER", "authorization".to_string());
        let bypass_paths = env::var("C2_GATEWAY_AUTH_BYPASS_PATHS")
            .unwrap_or_else(|_| "/health".to_string())
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();

        Self {
            api_token,
            header_name,
            bypass_paths,
        }
    }

    pub fn is_bypassed(&self, path: &str) -> bool {
        self.bypass_paths
            .iter()
            .any(|prefix| !prefix.is_empty() && path.starts_with(prefix))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub api: GatewayUpstream,
    pub web: GatewayUpstream,
    pub tls: Option<GatewayTlsConfig>,
    pub auth: GatewayAuthConfig,
}

impl GatewayConfig {
    pub fn from_env() -> Self {
        Self {
            api: GatewayUpstream::from_env("C2_GATEWAY_API", "c2-api", 8080),
            web: GatewayUpstream::from_env("C2_GATEWAY_WEB", "c2-web", 8080),
            tls: GatewayTlsConfig::from_env(),
            auth: GatewayAuthConfig::from_env(),
        }
    }
}

fn env_var(key: &str, default: String) -> String {
    env::var(key).unwrap_or(default)
}

fn env_var_u16(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(default)
}

fn env_var_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        })
        .unwrap_or(default)
}
