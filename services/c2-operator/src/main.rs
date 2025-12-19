use c2_config::ServiceConfig;
use c2_observability::{init, log_startup, ObservabilityConfig};
use c2_operator::{
    C2Cluster, C2ClusterSpec, C2ClusterStatus, DatabaseSpec, EnvVarSourceSpec, EnvVarSpec,
    ResourceRequirementsSpec, RuntimeSpec, ServiceSpec, ServiceStatus,
};
use futures_util::StreamExt;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    ConfigMapKeySelector, Container, ContainerPort, EnvVar, EnvVarSource, PodSpec, PodTemplateSpec,
    SecretKeySelector, Service, ServicePort, ServiceSpec as K8sServiceSpec,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::runtime::controller::{Action, Controller};
use kube::runtime::watcher;
use kube::{Api, Client, Resource, ResourceExt};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

type Context = Arc<ContextData>;

#[derive(Clone)]
struct ContextData {
    client: Client,
}

#[derive(Debug, Error)]
enum OperatorError {
    #[error("kubernetes client error: {0}")]
    Kube(#[from] kube::Error),
    #[error("missing namespace on C2Cluster")]
    MissingNamespace,
}

#[derive(Debug, Clone, Copy)]
enum Component {
    Api,
    Gateway,
    Web,
    Mcp,
    Worker,
}

impl Component {
    fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Gateway => "gateway",
            Self::Web => "web",
            Self::Mcp => "mcp",
            Self::Worker => "worker",
        }
    }

    fn image_name(self) -> &'static str {
        match self {
            Self::Api => "c2-api",
            Self::Gateway => "c2-gateway",
            Self::Web => "c2-web",
            Self::Mcp => "c2-mcp",
            Self::Worker => "c2-worker",
        }
    }

    fn default_port(self) -> u16 {
        8080
    }

    fn exposes_service(self) -> bool {
        !matches!(self, Self::Worker)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServiceConfig::from_env("c2-operator");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
        metrics_addr: config.metrics_addr.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let client = Client::try_default().await?;
    let context = Arc::new(ContextData { client: client.clone() });
    let api = Api::<C2Cluster>::all(client);

    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, context)
        .for_each(|result| async move {
            match result {
                Ok(_) => {}
                Err(err) => tracing::error!("reconcile error: {}", err),
            }
        })
        .await;

    Ok(())
}

async fn reconcile(cluster: Arc<C2Cluster>, context: Context) -> Result<Action, OperatorError> {
    let namespace = cluster
        .namespace()
        .ok_or(OperatorError::MissingNamespace)?;
    let name = cluster.name_any();
    let spec = &cluster.spec;
    let metrics_port = spec.runtime.metrics_port;

    let client = &context.client;
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let services: Api<Service> = Api::namespaced(client.clone(), &namespace);

    let components = [
        Component::Api,
        Component::Gateway,
        Component::Web,
        Component::Mcp,
        Component::Worker,
    ];

    for component in components {
        let service_spec = component_spec(spec, component);
        if service_spec
            .and_then(|value| value.enabled)
            .is_some_and(|value| !value)
        {
            continue;
        }

        let resource_name = resource_name(&name, component);
        let labels = component_labels(&name, component);
        let primary_port = service_port(component, service_spec);
        let image = resolve_image(spec, component, service_spec.and_then(|value| value.image.as_ref()));
        let image_pull_policy = spec.image.pull_policy.clone();
        let env = build_env(&cluster, component, primary_port, service_spec);
        let resources = service_spec
            .and_then(|value| value.resources.as_ref())
            .and_then(to_resource_requirements);
        let replicas = service_spec
            .and_then(|value| value.replicas)
            .unwrap_or(1);
        let node_selector = service_spec.and_then(|value| value.node_selector.clone());
        let container_ports = build_container_ports(component, primary_port, metrics_port);

        let deployment = deployment_resource(
            &namespace,
            &resource_name,
            &labels,
            component,
            &image,
            container_ports,
            image_pull_policy,
            env,
            replicas,
            resources,
            node_selector,
            cluster.controller_owner_ref(&()).map(|value| vec![value]),
        );

        apply_resource(&deployments, &resource_name, deployment).await?;

        if should_create_service(component, metrics_port) {
            let service_type = service_spec
                .and_then(|value| value.service.as_ref())
                .and_then(|value| value.service_type.clone());
            let service_ports = build_service_ports(component, primary_port, metrics_port);
            if service_ports.is_empty() {
                continue;
            }
            let service = service_resource(
                &namespace,
                &resource_name,
                &labels,
                service_ports,
                service_type,
                prometheus_annotations(metrics_port),
                cluster.controller_owner_ref(&()).map(|value| vec![value]),
            );
            apply_resource(&services, &resource_name, service).await?;
        }
    }

    update_status(&cluster, &deployments, client).await?;

    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(_cluster: Arc<C2Cluster>, _error: &OperatorError, _ctx: Context) -> Action {
    Action::requeue(Duration::from_secs(30))
}

fn component_spec<'a>(spec: &'a C2ClusterSpec, component: Component) -> Option<&'a ServiceSpec> {
    match component {
        Component::Api => spec.services.api.as_ref(),
        Component::Gateway => spec.services.gateway.as_ref(),
        Component::Web => spec.services.web.as_ref(),
        Component::Mcp => spec.services.mcp.as_ref(),
        Component::Worker => spec.services.worker.as_ref(),
    }
}

fn resource_name(cluster: &str, component: Component) -> String {
    format!("{cluster}-{}", component.as_str())
}

fn service_port(component: Component, spec: Option<&ServiceSpec>) -> u16 {
    spec.and_then(|value| value.service.as_ref())
        .and_then(|value| value.port)
        .unwrap_or_else(|| component.default_port())
}

fn should_create_service(component: Component, metrics_port: Option<u16>) -> bool {
    component.exposes_service() || metrics_port.is_some()
}

fn build_container_ports(
    component: Component,
    primary_port: u16,
    metrics_port: Option<u16>,
) -> Vec<ContainerPort> {
    let mut ports = Vec::new();
    if component.exposes_service() {
        ports.push(ContainerPort {
            container_port: primary_port.into(),
            name: Some("http".to_string()),
            ..Default::default()
        });
    }
    if let Some(metrics_port) = metrics_port {
        let metrics_port = i32::from(metrics_port);
        if ports.iter().any(|port| port.container_port == metrics_port) {
            return ports;
        }
        ports.push(ContainerPort {
            container_port: metrics_port,
            name: Some("metrics".to_string()),
            ..Default::default()
        });
    }
    ports
}

fn build_service_ports(
    component: Component,
    primary_port: u16,
    metrics_port: Option<u16>,
) -> Vec<ServicePort> {
    let mut ports = Vec::new();
    if component.exposes_service() {
        ports.push(ServicePort {
            name: Some("http".to_string()),
            port: primary_port.into(),
            target_port: Some(IntOrString::Int(primary_port.into())),
            ..Default::default()
        });
    }
    if let Some(metrics_port) = metrics_port {
        let metrics_port = i32::from(metrics_port);
        if ports.iter().any(|port| port.port == metrics_port) {
            return ports;
        }
        ports.push(ServicePort {
            name: Some("metrics".to_string()),
            port: metrics_port,
            target_port: Some(IntOrString::Int(metrics_port)),
            ..Default::default()
        });
    }
    ports
}

fn prometheus_annotations(metrics_port: Option<u16>) -> Option<BTreeMap<String, String>> {
    let metrics_port = metrics_port?;
    let mut annotations = BTreeMap::new();
    annotations.insert("prometheus.io/scrape".to_string(), "true".to_string());
    annotations.insert("prometheus.io/port".to_string(), metrics_port.to_string());
    annotations.insert("prometheus.io/path".to_string(), "/metrics".to_string());
    Some(annotations)
}

fn resolve_image(spec: &C2ClusterSpec, component: Component, override_image: Option<&String>) -> String {
    if let Some(image) = override_image {
        return image.clone();
    }
    let per_service = match component {
        Component::Api => spec.image.api.as_ref(),
        Component::Gateway => spec.image.gateway.as_ref(),
        Component::Web => spec.image.web.as_ref(),
        Component::Mcp => spec.image.mcp.as_ref(),
        Component::Worker => spec.image.worker.as_ref(),
    };
    if let Some(image) = per_service {
        return image.clone();
    }
    let tag = spec
        .image
        .tag
        .clone()
        .unwrap_or_else(|| spec.version.clone());
    match spec.image.registry.as_deref() {
        Some(registry) if !registry.is_empty() => {
            format!("{}/{}:{tag}", registry.trim_end_matches('/'), component.image_name())
        }
        _ => format!("{}:{tag}", component.image_name()),
    }
}

fn build_env(
    cluster: &C2Cluster,
    component: Component,
    port: u16,
    service_spec: Option<&ServiceSpec>,
) -> Vec<EnvVar> {
    let mut vars = BTreeMap::<String, EnvVar>::new();
    let service_name = resource_name(&cluster.name_any(), component);
    insert_env(
        &mut vars,
        EnvVar {
            name: "C2_SERVICE_NAME".to_string(),
            value: Some(service_name.clone()),
            value_from: None,
        },
    );
    insert_env(
        &mut vars,
        EnvVar {
            name: "C2_BIND_ADDR".to_string(),
            value: Some(format!("0.0.0.0:{port}")),
            value_from: None,
        },
    );

    apply_runtime_env(&mut vars, &cluster.spec.runtime);
    apply_database_env(&mut vars, cluster.spec.database.as_ref());

    if matches!(component, Component::Gateway) {
        let api_name = resource_name(&cluster.name_any(), Component::Api);
        let web_name = resource_name(&cluster.name_any(), Component::Web);
        insert_env(
            &mut vars,
            EnvVar {
                name: "C2_GATEWAY_API_HOST".to_string(),
                value: Some(api_name),
                value_from: None,
            },
        );
        insert_env(
            &mut vars,
            EnvVar {
                name: "C2_GATEWAY_API_PORT".to_string(),
                value: Some(service_port(Component::Api, component_spec(&cluster.spec, Component::Api)).to_string()),
                value_from: None,
            },
        );
        insert_env(
            &mut vars,
            EnvVar {
                name: "C2_GATEWAY_WEB_HOST".to_string(),
                value: Some(web_name),
                value_from: None,
            },
        );
        insert_env(
            &mut vars,
            EnvVar {
                name: "C2_GATEWAY_WEB_PORT".to_string(),
                value: Some(service_port(Component::Web, component_spec(&cluster.spec, Component::Web)).to_string()),
                value_from: None,
            },
        );
    }

    for env in &cluster.spec.global_env {
        insert_env(&mut vars, to_env_var(env));
    }
    if let Some(service_env) = service_spec.and_then(|value| value.env.as_ref()) {
        for env in service_env {
            insert_env(&mut vars, to_env_var(env));
        }
    }

    vars.into_values().collect()
}

fn apply_runtime_env(vars: &mut BTreeMap<String, EnvVar>, runtime: &RuntimeSpec) {
    if let Some(environment) = runtime.environment.as_ref() {
        insert_env(
            vars,
            EnvVar {
                name: "C2_ENV".to_string(),
                value: Some(environment.clone()),
                value_from: None,
            },
        );
    }
    if let Some(region) = runtime.region.as_ref() {
        insert_env(
            vars,
            EnvVar {
                name: "C2_REGION".to_string(),
                value: Some(region.clone()),
                value_from: None,
            },
        );
    }
    if let Some(log_level) = runtime.log_level.as_ref() {
        insert_env(
            vars,
            EnvVar {
                name: "C2_LOG_LEVEL".to_string(),
                value: Some(log_level.clone()),
                value_from: None,
            },
        );
    }
    if let Some(data_dir) = runtime.data_dir.as_ref() {
        insert_env(
            vars,
            EnvVar {
                name: "C2_DATA_DIR".to_string(),
                value: Some(data_dir.clone()),
                value_from: None,
            },
        );
    }
    if let Some(trusted) = runtime.trusted_proxies.as_ref() {
        let value = trusted.join(",");
        if !value.is_empty() {
            insert_env(
                vars,
                EnvVar {
                    name: "C2_TRUSTED_PROXIES".to_string(),
                    value: Some(value),
                    value_from: None,
                },
            );
        }
    }
    if let Some(metrics_port) = runtime.metrics_port {
        insert_env(
            vars,
            EnvVar {
                name: "C2_METRICS_ADDR".to_string(),
                value: Some(format!("0.0.0.0:{metrics_port}")),
                value_from: None,
            },
        );
    }
}

fn apply_database_env(vars: &mut BTreeMap<String, EnvVar>, database: Option<&DatabaseSpec>) {
    let Some(database) = database else {
        return;
    };
    if let Some(surreal) = database.surreal.as_ref() {
        if let Some(endpoint) = surreal.endpoint.as_ref() {
            insert_env(vars, env_value("C2_SURREAL_ENDPOINT", endpoint));
        }
        if let Some(namespace) = surreal.namespace.as_ref() {
            insert_env(vars, env_value("C2_SURREAL_NAMESPACE", namespace));
        }
        if let Some(database) = surreal.database.as_ref() {
            insert_env(vars, env_value("C2_SURREAL_DATABASE", database));
        }
        if let Some(username) = surreal.username.as_ref() {
            insert_env(vars, env_value("C2_SURREAL_USERNAME", username));
        }
        if let Some(password) = surreal.password.as_ref() {
            insert_env(vars, env_value("C2_SURREAL_PASSWORD", password));
        }
    }
    if let Some(postgres) = database.postgres.as_ref() {
        if let Some(url) = postgres.url.as_ref() {
            insert_env(vars, env_value("C2_POSTGRES_URL", url));
        }
        if let Some(max) = postgres.max_connections {
            insert_env(vars, env_value("C2_POSTGRES_MAX_CONNECTIONS", &max.to_string()));
        }
    }
    if let Some(timescale) = database.timescale.as_ref() {
        if let Some(url) = timescale.url.as_ref() {
            insert_env(vars, env_value("C2_TIMESCALE_URL", url));
        }
        if let Some(max) = timescale.max_connections {
            insert_env(
                vars,
                env_value("C2_TIMESCALE_MAX_CONNECTIONS", &max.to_string()),
            );
        }
    }
}

fn env_value(name: &str, value: &str) -> EnvVar {
    EnvVar {
        name: name.to_string(),
        value: Some(value.to_string()),
        value_from: None,
    }
}

fn insert_env(vars: &mut BTreeMap<String, EnvVar>, env: EnvVar) {
    vars.insert(env.name.clone(), env);
}

fn to_env_var(env: &EnvVarSpec) -> EnvVar {
    EnvVar {
        name: env.name.clone(),
        value: env.value.clone(),
        value_from: env.value_from.as_ref().map(to_env_source),
    }
}

fn to_env_source(source: &EnvVarSourceSpec) -> EnvVarSource {
    EnvVarSource {
        config_map_key_ref: source.config_map_key_ref.as_ref().map(|value| ConfigMapKeySelector {
            name: value.name.clone(),
            key: value.key.clone(),
            optional: value.optional,
        }),
        secret_key_ref: source.secret_key_ref.as_ref().map(|value| SecretKeySelector {
            name: value.name.clone(),
            key: value.key.clone(),
            optional: value.optional,
        }),
        field_ref: None,
        resource_field_ref: None,
    }
}

fn component_labels(cluster: &str, component: Component) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("app.kubernetes.io/name".to_string(), "c2".to_string());
    labels.insert("app.kubernetes.io/instance".to_string(), cluster.to_string());
    labels.insert(
        "app.kubernetes.io/component".to_string(),
        component.as_str().to_string(),
    );
    labels.insert(
        "app.kubernetes.io/managed-by".to_string(),
        "c2-operator".to_string(),
    );
    labels
}

fn deployment_resource(
    namespace: &str,
    name: &str,
    labels: &BTreeMap<String, String>,
    component: Component,
    image: &str,
    container_ports: Vec<ContainerPort>,
    image_pull_policy: Option<String>,
    env: Vec<EnvVar>,
    replicas: i32,
    resources: Option<k8s_openapi::api::core::v1::ResourceRequirements>,
    node_selector: Option<BTreeMap<String, String>>,
    owner_references: Option<Vec<k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference>>,
) -> Deployment {
    Deployment {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            owner_references,
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(replicas),
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels.clone()),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: component.as_str().to_string(),
                        image: Some(image.to_string()),
                        image_pull_policy,
                        ports: if container_ports.is_empty() {
                            None
                        } else {
                            Some(container_ports)
                        },
                        env: Some(env),
                        resources,
                        ..Default::default()
                    }],
                    node_selector,
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn service_resource(
    namespace: &str,
    name: &str,
    labels: &BTreeMap<String, String>,
    ports: Vec<ServicePort>,
    service_type: Option<String>,
    annotations: Option<BTreeMap<String, String>>,
    owner_references: Option<Vec<k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference>>,
) -> Service {
    Service {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            annotations,
            owner_references,
            ..Default::default()
        },
        spec: Some(K8sServiceSpec {
            selector: Some(labels.clone()),
            ports: Some(ports),
            type_: service_type,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn to_resource_requirements(
    spec: &ResourceRequirementsSpec,
) -> Option<k8s_openapi::api::core::v1::ResourceRequirements> {
    let limits = spec
        .limits
        .as_ref()
        .map(map_resource_quantities);
    let requests = spec
        .requests
        .as_ref()
        .map(map_resource_quantities);
    if limits.is_none() && requests.is_none() {
        return None;
    }
    Some(k8s_openapi::api::core::v1::ResourceRequirements {
        limits,
        requests,
        claims: None,
    })
}

fn map_resource_quantities(
    values: &BTreeMap<String, String>,
) -> BTreeMap<String, Quantity> {
    values
        .iter()
        .map(|(key, value)| (key.clone(), Quantity(value.clone())))
        .collect()
}

async fn apply_resource<K>(
    api: &Api<K>,
    name: &str,
    resource: K,
) -> Result<(), OperatorError>
where
    K: kube::Resource + Clone + serde::Serialize + DeserializeOwned + std::fmt::Debug,
    <K as kube::Resource>::DynamicType: Default,
{
    let params = kube::api::PatchParams::apply("c2-operator").force();
    api.patch(name, &params, &kube::api::Patch::Apply(&resource))
        .await?;
    Ok(())
}

async fn update_status(
    cluster: &C2Cluster,
    deployments: &Api<Deployment>,
    client: &Client,
) -> Result<(), OperatorError> {
    let namespace = cluster
        .namespace()
        .ok_or(OperatorError::MissingNamespace)?;
    let name = cluster.name_any();
    let mut service_states = Vec::new();
    let components = [
        Component::Api,
        Component::Gateway,
        Component::Web,
        Component::Mcp,
        Component::Worker,
    ];
    for component in components {
        let service_spec = component_spec(&cluster.spec, component);
        if service_spec
            .and_then(|value| value.enabled)
            .is_some_and(|value| !value)
        {
            continue;
        }
        let deployment_name = resource_name(&name, component);
        let deployment = deployments.get_opt(&deployment_name).await?;
        let ready = deployment
            .as_ref()
            .and_then(|value| value.status.as_ref())
            .and_then(|status| status.ready_replicas)
            .unwrap_or(0);
        service_states.push(ServiceStatus {
            name: deployment_name,
            ready_replicas: Some(ready),
        });
    }
    let ready = service_states.iter().all(|status| status.ready_replicas.unwrap_or(0) > 0);
    let status = C2ClusterStatus {
        phase: Some(if ready { "Ready" } else { "Reconciling" }.to_string()),
        ready: Some(ready),
        observed_generation: cluster.metadata.generation,
        last_reconcile_time: Some(now_epoch_seconds()),
        services: Some(service_states),
    };

    let crd_api: Api<C2Cluster> = Api::namespaced(client.clone(), &namespace);
    let params = kube::api::PatchParams::apply("c2-operator");
    let status_patch = serde_json::json!({ "status": status });
    crd_api
        .patch_status(&name, &params, &kube::api::Patch::Merge(&status_patch))
        .await?;
    Ok(())
}

fn now_epoch_seconds() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    seconds.to_string()
}
