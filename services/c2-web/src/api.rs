use c2_core::{
    now_epoch_millis, Asset, AssetStatus, Incident, IncidentStatus, Mission, MissionStatus, Unit,
};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

#[derive(Debug)]
pub struct ApiError {
    pub message: String,
}

impl ApiError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        Self::new(error.to_string())
    }
}

#[derive(Clone)]
struct ApiAuth {
    tenant_id: String,
    headers: HeaderMap,
}

impl ApiAuth {
    fn from_env() -> Option<Self> {
        let tenant_id = env::var("C2_UI_TENANT_ID").ok()?;
        let user_id = env::var("C2_UI_USER_ID").ok()?;
        let roles = env::var("C2_UI_ROLES").ok()?;
        let permissions = env::var("C2_UI_PERMISSIONS").ok()?;
        let clearance =
            env::var("C2_UI_CLEARANCE").unwrap_or_else(|_| "unclassified".to_string());

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-c2-tenant-id",
            HeaderValue::from_str(&tenant_id).ok()?,
        );
        headers.insert("x-c2-user-id", HeaderValue::from_str(&user_id).ok()?);
        headers.insert("x-c2-roles", HeaderValue::from_str(&roles).ok()?);
        headers.insert(
            "x-c2-permissions",
            HeaderValue::from_str(&permissions).ok()?,
        );
        let clearance_value = HeaderValue::from_str(&clearance)
            .unwrap_or_else(|_| HeaderValue::from_static("unclassified"));
        headers.insert("x-c2-clearance", clearance_value);

        Some(Self { tenant_id, headers })
    }
}

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: Url,
    auth: Option<ApiAuth>,
    poll_interval: Duration,
    list_limit: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusResponse {
    pub service: String,
    pub environment: String,
    pub region: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct MissionSummary {
    pub total: usize,
    pub active: usize,
    pub planned: usize,
    pub suspended: usize,
    pub completed: usize,
    pub aborted: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct AssetSummary {
    pub total: usize,
    pub ready: usize,
    pub degraded: usize,
    pub maintenance: usize,
    pub lost: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct IncidentSummary {
    pub total: usize,
    pub active: usize,
    pub responding: usize,
    pub resolved: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct UiSnapshot {
    pub timestamp_ms: u64,
    pub missions: MissionSummary,
    pub assets: AssetSummary,
    pub incidents: IncidentSummary,
}

impl UiSnapshot {
    pub fn empty() -> Self {
        Self {
            timestamp_ms: now_epoch_millis(),
            missions: MissionSummary {
                total: 0,
                active: 0,
                planned: 0,
                suspended: 0,
                completed: 0,
                aborted: 0,
            },
            assets: AssetSummary {
                total: 0,
                ready: 0,
                degraded: 0,
                maintenance: 0,
                lost: 0,
            },
            incidents: IncidentSummary {
                total: 0,
                active: 0,
                responding: 0,
                resolved: 0,
            },
        }
    }

    pub fn from_entities(entities: &UiEntitySnapshot) -> Self {
        Self {
            timestamp_ms: entities.timestamp_ms,
            missions: summarize_missions(&entities.missions),
            assets: summarize_assets(&entities.assets),
            incidents: summarize_incidents(&entities.incidents),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct UiEntitySnapshot {
    pub timestamp_ms: u64,
    pub missions: Vec<Mission>,
    pub assets: Vec<Asset>,
    pub incidents: Vec<Incident>,
    pub units: Vec<Unit>,
}

impl UiEntitySnapshot {
    pub fn empty() -> Self {
        Self {
            timestamp_ms: now_epoch_millis(),
            missions: Vec::new(),
            assets: Vec::new(),
            incidents: Vec::new(),
            units: Vec::new(),
        }
    }
}

impl ApiClient {
    pub fn from_env() -> Result<Self, ApiError> {
        let base = env::var("C2_API_BASE_URL").unwrap_or_else(|_| "http://c2-api:8080".to_string());
        let base = format!("{}/", base.trim_end_matches('/'));
        let base_url = Url::parse(&base).map_err(|err| ApiError::new(err.to_string()))?;
        let poll_interval_ms = env_var_u64("C2_UI_POLL_INTERVAL_MS", 2000);
        let list_limit = env_var_usize("C2_UI_LIST_LIMIT", 200);
        Ok(Self {
            client: Client::new(),
            base_url,
            auth: ApiAuth::from_env(),
            poll_interval: Duration::from_millis(poll_interval_ms),
            list_limit: list_limit.max(10),
        })
    }

    pub fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    pub fn auth_enabled(&self) -> bool {
        self.auth.is_some()
    }

    pub async fn status(&self) -> Result<StatusResponse, ApiError> {
        let url = self
            .base_url
            .join("v1/status")
            .map_err(|err| ApiError::new(err.to_string()))?;
        let mut request = self.client.get(url);
        if let Some(auth) = &self.auth {
            request = request.headers(auth.headers.clone());
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(ApiError::new(format!(
                "status request failed with {}",
                response.status()
            )));
        }
        Ok(response.json::<StatusResponse>().await?)
    }

    pub async fn snapshot(&self) -> Result<UiSnapshot, ApiError> {
        let entities = self.entities().await?;
        Ok(UiSnapshot::from_entities(&entities))
    }

    pub async fn entities(&self) -> Result<UiEntitySnapshot, ApiError> {
        let auth = self
            .auth
            .as_ref()
            .ok_or_else(|| ApiError::new("missing C2_UI_* auth configuration"))?;
        let missions = self
            .list_missions(auth, self.list_limit, 0)
            .await
            .unwrap_or_default();
        let assets = self
            .list_assets(auth, self.list_limit, 0)
            .await
            .unwrap_or_default();
        let incidents = self
            .list_incidents(auth, self.list_limit, 0)
            .await
            .unwrap_or_default();
        let units = self
            .list_units(auth, self.list_limit, 0)
            .await
            .unwrap_or_default();

        Ok(UiEntitySnapshot {
            timestamp_ms: now_epoch_millis(),
            missions,
            assets,
            incidents,
            units,
        })
    }

    async fn list_missions(
        &self,
        auth: &ApiAuth,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Mission>, ApiError> {
        let url = self
            .base_url
            .join("v1/missions")
            .map_err(|err| ApiError::new(err.to_string()))?;
        let response = self
            .client
            .get(url)
            .headers(auth.headers.clone())
            .query(&[
                ("tenant_id", auth.tenant_id.as_str()),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ApiError::new(format!(
                "missions request failed with {}",
                response.status()
            )));
        }
        Ok(response.json::<Vec<Mission>>().await?)
    }

    async fn list_assets(
        &self,
        auth: &ApiAuth,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Asset>, ApiError> {
        let url = self
            .base_url
            .join("v1/assets")
            .map_err(|err| ApiError::new(err.to_string()))?;
        let response = self
            .client
            .get(url)
            .headers(auth.headers.clone())
            .query(&[
                ("tenant_id", auth.tenant_id.as_str()),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ApiError::new(format!(
                "assets request failed with {}",
                response.status()
            )));
        }
        Ok(response.json::<Vec<Asset>>().await?)
    }

    async fn list_incidents(
        &self,
        auth: &ApiAuth,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Incident>, ApiError> {
        let url = self
            .base_url
            .join("v1/incidents")
            .map_err(|err| ApiError::new(err.to_string()))?;
        let response = self
            .client
            .get(url)
            .headers(auth.headers.clone())
            .query(&[
                ("tenant_id", auth.tenant_id.as_str()),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ApiError::new(format!(
                "incidents request failed with {}",
                response.status()
            )));
        }
        Ok(response.json::<Vec<Incident>>().await?)
    }

    async fn list_units(
        &self,
        auth: &ApiAuth,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Unit>, ApiError> {
        let url = self
            .base_url
            .join("v1/units")
            .map_err(|err| ApiError::new(err.to_string()))?;
        let response = self
            .client
            .get(url)
            .headers(auth.headers.clone())
            .query(&[
                ("tenant_id", auth.tenant_id.as_str()),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ApiError::new(format!(
                "units request failed with {}",
                response.status()
            )));
        }
        Ok(response.json::<Vec<Unit>>().await?)
    }
}

fn summarize_missions(missions: &[Mission]) -> MissionSummary {
    let mut summary = MissionSummary {
        total: missions.len(),
        active: 0,
        planned: 0,
        suspended: 0,
        completed: 0,
        aborted: 0,
    };
    for mission in missions {
        match mission.status {
            MissionStatus::Active => summary.active += 1,
            MissionStatus::Planned => summary.planned += 1,
            MissionStatus::Suspended => summary.suspended += 1,
            MissionStatus::Completed => summary.completed += 1,
            MissionStatus::Aborted => summary.aborted += 1,
        }
    }
    summary
}

fn summarize_assets(assets: &[Asset]) -> AssetSummary {
    let mut summary = AssetSummary {
        total: assets.len(),
        ready: 0,
        degraded: 0,
        maintenance: 0,
        lost: 0,
    };
    for asset in assets {
        match asset.status {
            AssetStatus::Available | AssetStatus::Assigned => summary.ready += 1,
            AssetStatus::Degraded => summary.degraded += 1,
            AssetStatus::Maintenance => summary.maintenance += 1,
            AssetStatus::Lost => summary.lost += 1,
        }
    }
    summary
}

fn summarize_incidents(incidents: &[Incident]) -> IncidentSummary {
    let mut summary = IncidentSummary {
        total: incidents.len(),
        active: 0,
        responding: 0,
        resolved: 0,
    };
    for incident in incidents {
        match incident.status {
            IncidentStatus::Reported | IncidentStatus::Verified | IncidentStatus::Responding => {
                summary.active += 1
            }
            IncidentStatus::Contained | IncidentStatus::Resolved | IncidentStatus::Closed => {
                summary.resolved += 1
            }
        }
        if incident.status == IncidentStatus::Responding {
            summary.responding += 1;
        }
    }
    summary
}

fn env_var_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_var_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}
