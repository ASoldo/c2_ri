use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "c2.walaris.com",
    version = "v1alpha1",
    kind = "C2Cluster",
    plural = "c2clusters",
    namespaced,
    status = "C2ClusterStatus",
    shortname = "c2"
)]
#[serde(rename_all = "camelCase")]
pub struct C2ClusterSpec {
    pub version: String,
    #[serde(default)]
    pub image: ImageSpec,
    #[serde(default)]
    pub runtime: RuntimeSpec,
    #[serde(default)]
    pub services: C2ServicesSpec,
    #[serde(default)]
    pub global_env: Vec<EnvVarSpec>,
    pub database: Option<DatabaseSpec>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageSpec {
    pub registry: Option<String>,
    pub tag: Option<String>,
    pub pull_policy: Option<String>,
    pub api: Option<String>,
    pub gateway: Option<String>,
    pub web: Option<String>,
    pub mcp: Option<String>,
    pub worker: Option<String>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSpec {
    pub environment: Option<String>,
    pub region: Option<String>,
    pub log_level: Option<String>,
    pub data_dir: Option<String>,
    pub trusted_proxies: Option<Vec<String>>,
    pub metrics_port: Option<u16>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct C2ServicesSpec {
    pub api: Option<ServiceSpec>,
    pub gateway: Option<ServiceSpec>,
    pub web: Option<ServiceSpec>,
    pub mcp: Option<ServiceSpec>,
    pub worker: Option<ServiceSpec>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSpec {
    pub enabled: Option<bool>,
    pub replicas: Option<i32>,
    pub image: Option<String>,
    pub env: Option<Vec<EnvVarSpec>>,
    pub service: Option<ServiceExposure>,
    pub resources: Option<ResourceRequirementsSpec>,
    pub node_selector: Option<BTreeMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceExposure {
    pub service_type: Option<String>,
    pub port: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirementsSpec {
    pub limits: Option<BTreeMap<String, String>>,
    pub requests: Option<BTreeMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarSpec {
    pub name: String,
    pub value: Option<String>,
    pub value_from: Option<EnvVarSourceSpec>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarSourceSpec {
    pub config_map_key_ref: Option<ConfigMapKeyRefSpec>,
    pub secret_key_ref: Option<SecretKeyRefSpec>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapKeyRefSpec {
    pub name: String,
    pub key: String,
    pub optional: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretKeyRefSpec {
    pub name: String,
    pub key: String,
    pub optional: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSpec {
    pub surreal: Option<SurrealDbSpec>,
    pub postgres: Option<PostgresSpec>,
    pub timescale: Option<TimescaleSpec>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SurrealDbSpec {
    pub endpoint: Option<String>,
    pub namespace: Option<String>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PostgresSpec {
    pub url: Option<String>,
    pub max_connections: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimescaleSpec {
    pub url: Option<String>,
    pub max_connections: Option<u32>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct C2ClusterStatus {
    pub phase: Option<String>,
    pub ready: Option<bool>,
    pub observed_generation: Option<i64>,
    pub last_reconcile_time: Option<String>,
    pub services: Option<Vec<ServiceStatus>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub name: String,
    pub ready_replicas: Option<i32>,
}
