use async_trait::async_trait;
use c2_core::{
    Asset, AssetId, AssetKind, AssetStatus, Capability, CapabilityId, CommsStatus, Incident,
    IncidentId, IncidentStatus, IncidentType, MaintenanceState, Mission, MissionId, MissionStatus,
    OperationalPriority, ReadinessState, SecurityClassification, Task, TaskId, TaskStatus, Team,
    TeamId, TenantId, Unit, UnitId,
};
use c2_storage::{
    AssetRepository, CapabilityRepository, IncidentRepository, MissionRepository, StorageError,
    TaskRepository, TeamRepository, UnitRepository,
};
use serde::{Deserialize, Serialize};
use std::env;
use surrealdb::engine::remote::ws::{Client, Ws, Wss};
use surrealdb::opt::auth::Root;
use surrealdb::sql::{Id, Thing};
use surrealdb::Surreal;
use tokio::time::{sleep, Duration};
use tracing::warn;
use uuid::Uuid;

const TABLE_MISSION: &str = "mission";
const TABLE_ASSET: &str = "asset";
const TABLE_UNIT: &str = "unit";
const TABLE_TEAM: &str = "team";
const TABLE_CAPABILITY: &str = "capability";
const TABLE_INCIDENT: &str = "incident";
const TABLE_TASK: &str = "task";
const SURREAL_SCHEMA: &str = include_str!("../schema/c2.surql");

#[derive(Debug, Clone)]
pub struct SurrealConfig {
    pub endpoint: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub connect_retry_initial_ms: u64,
    pub connect_retry_max_ms: u64,
    pub connect_retry_max_attempts: u32,
}

impl SurrealConfig {
    pub fn from_env() -> Self {
        Self {
            endpoint: env_var("C2_SURREAL_ENDPOINT", "127.0.0.1:8000".to_string()),
            namespace: env_var("C2_SURREAL_NAMESPACE", "c2".to_string()),
            database: env_var("C2_SURREAL_DATABASE", "operations".to_string()),
            username: env_var("C2_SURREAL_USERNAME", "root".to_string()),
            password: env_var("C2_SURREAL_PASSWORD", "root".to_string()),
            connect_retry_initial_ms: env_var_u64("C2_SURREAL_CONNECT_RETRY_INITIAL_MS", 500),
            connect_retry_max_ms: env_var_u64("C2_SURREAL_CONNECT_RETRY_MAX_MS", 5000),
            connect_retry_max_attempts: env_var_u32("C2_SURREAL_CONNECT_RETRY_MAX_ATTEMPTS", 0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SurrealStore {
    #[allow(dead_code)]
    db: Surreal<Client>,
}

#[derive(Debug, Clone, Copy)]
enum SurrealScheme {
    Ws,
    Wss,
}

fn normalize_endpoint(raw: &str) -> Result<(SurrealScheme, String), StorageError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(StorageError::new("C2_SURREAL_ENDPOINT is empty"));
    }
    if let Some(stripped) = trimmed.strip_prefix("ws://") {
        return Ok((SurrealScheme::Ws, stripped.to_string()));
    }
    if let Some(stripped) = trimmed.strip_prefix("wss://") {
        return Ok((SurrealScheme::Wss, stripped.to_string()));
    }
    if let Some((scheme, rest)) = trimmed.split_once("://") {
        warn!(
            scheme = scheme,
            "Unsupported SurrealDB endpoint scheme, defaulting to ws"
        );
        return Ok((SurrealScheme::Ws, rest.to_string()));
    }
    Ok((SurrealScheme::Ws, trimmed.to_string()))
}

#[derive(Debug, Deserialize)]
struct SurrealMissionRecord {
    id: Thing,
    tenant_id: String,
    name: String,
    status: MissionStatus,
    priority: OperationalPriority,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealMissionWrite {
    tenant_id: String,
    name: String,
    status: MissionStatus,
    priority: OperationalPriority,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SurrealAssetRecord {
    id: Thing,
    tenant_id: String,
    name: String,
    kind: AssetKind,
    status: AssetStatus,
    #[serde(default)]
    readiness: ReadinessState,
    #[serde(default)]
    comms_status: CommsStatus,
    #[serde(default)]
    maintenance_state: MaintenanceState,
    #[serde(default)]
    unit_id: Option<String>,
    #[serde(default)]
    capability_ids: Vec<String>,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealAssetWrite {
    tenant_id: String,
    name: String,
    kind: AssetKind,
    status: AssetStatus,
    readiness: ReadinessState,
    comms_status: CommsStatus,
    maintenance_state: MaintenanceState,
    unit_id: Option<String>,
    capability_ids: Vec<String>,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SurrealUnitRecord {
    id: Thing,
    tenant_id: String,
    display_name: String,
    #[serde(default)]
    callsign: Option<String>,
    classification: SecurityClassification,
    #[serde(default)]
    readiness: ReadinessState,
    #[serde(default)]
    comms_status: CommsStatus,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(default)]
    capability_ids: Vec<String>,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealUnitWrite {
    tenant_id: String,
    display_name: String,
    callsign: Option<String>,
    classification: SecurityClassification,
    readiness: ReadinessState,
    comms_status: CommsStatus,
    team_id: Option<String>,
    capability_ids: Vec<String>,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SurrealTeamRecord {
    id: Thing,
    tenant_id: String,
    name: String,
    #[serde(default)]
    callsign: Option<String>,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealTeamWrite {
    tenant_id: String,
    name: String,
    callsign: Option<String>,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SurrealCapabilityRecord {
    id: Thing,
    tenant_id: String,
    code: String,
    name: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    description: Option<String>,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealCapabilityWrite {
    tenant_id: String,
    code: String,
    name: String,
    category: Option<String>,
    description: Option<String>,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SurrealIncidentRecord {
    id: Thing,
    tenant_id: String,
    incident_type: IncidentType,
    status: IncidentStatus,
    summary: String,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealIncidentWrite {
    tenant_id: String,
    incident_type: IncidentType,
    status: IncidentStatus,
    summary: String,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SurrealTaskRecord {
    id: Thing,
    tenant_id: String,
    mission_id: String,
    title: String,
    status: TaskStatus,
    priority: OperationalPriority,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealTaskWrite {
    tenant_id: String,
    mission_id: String,
    title: String,
    status: TaskStatus,
    priority: OperationalPriority,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl SurrealStore {
    pub async fn connect(config: &SurrealConfig) -> Result<Self, StorageError> {
        let (scheme, endpoint) = normalize_endpoint(&config.endpoint)?;
        let db = match scheme {
            SurrealScheme::Ws => Surreal::new::<Ws>(&endpoint).await.map_err(map_err)?,
            SurrealScheme::Wss => Surreal::new::<Wss>(&endpoint).await.map_err(map_err)?,
        };
        db.signin(Root {
            username: &config.username,
            password: &config.password,
        })
        .await
        .map_err(map_err)?;
        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .map_err(map_err)?;
        apply_schema(&db).await?;
        Ok(Self { db })
    }

    pub async fn connect_with_retry(config: &SurrealConfig) -> Result<Self, StorageError> {
        let mut attempt: u32 = 0;
        let mut delay_ms = config.connect_retry_initial_ms.max(1);
        let max_delay = config.connect_retry_max_ms.max(delay_ms);

        loop {
            match Self::connect(config).await {
                Ok(store) => return Ok(store),
                Err(err) => {
                    attempt = attempt.saturating_add(1);
                    if config.connect_retry_max_attempts > 0
                        && attempt >= config.connect_retry_max_attempts
                    {
                        return Err(err);
                    }
                    warn!(
                        attempt,
                        delay_ms,
                        error = %err,
                        "SurrealDB connection failed, retrying"
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = delay_ms.saturating_mul(2).min(max_delay);
                }
            }
        }
    }
}

async fn apply_schema(db: &Surreal<Client>) -> Result<(), StorageError> {
    db.query(SURREAL_SCHEMA).await.map_err(map_err)?;
    Ok(())
}

#[async_trait]
impl MissionRepository for SurrealStore {
    async fn get(&self, id: MissionId) -> Result<Option<Mission>, StorageError> {
        let record: Option<SurrealMissionRecord> = self
            .db
            .select((TABLE_MISSION, id.to_string()))
            .await
            .map_err(map_err)?;
        match record {
            Some(record) => Ok(Some(record.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Mission>, StorageError> {
        #[derive(Serialize)]
        struct Bindings {
            tenant_id: String,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM mission WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id: tenant_id.to_string(),
                limit,
                offset,
            })
            .await
            .map_err(map_err)?;

        let records: Vec<SurrealMissionRecord> = response.take(0).map_err(map_err)?;
        records
            .into_iter()
            .map(Mission::try_from)
            .collect()
    }

    async fn upsert(&self, mission: Mission) -> Result<(), StorageError> {
        let record = SurrealMissionWrite::from(&mission);
        let _: Option<SurrealMissionRecord> = self
            .db
            .upsert((TABLE_MISSION, mission.id.to_string()))
            .content(record)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: MissionId) -> Result<(), StorageError> {
        let _: Option<SurrealMissionRecord> = self
            .db
            .delete((TABLE_MISSION, id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl AssetRepository for SurrealStore {
    async fn get(&self, id: AssetId) -> Result<Option<Asset>, StorageError> {
        let record: Option<SurrealAssetRecord> = self
            .db
            .select((TABLE_ASSET, id.to_string()))
            .await
            .map_err(map_err)?;
        match record {
            Some(record) => Ok(Some(record.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Asset>, StorageError> {
        #[derive(Serialize)]
        struct Bindings {
            tenant_id: String,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM asset WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id: tenant_id.to_string(),
                limit,
                offset,
            })
            .await
            .map_err(map_err)?;

        let records: Vec<SurrealAssetRecord> = response.take(0).map_err(map_err)?;
        records.into_iter().map(Asset::try_from).collect()
    }

    async fn upsert(&self, asset: Asset) -> Result<(), StorageError> {
        let record = SurrealAssetWrite::from(&asset);
        let _: Option<SurrealAssetRecord> = self
            .db
            .upsert((TABLE_ASSET, asset.id.to_string()))
            .content(record)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: AssetId) -> Result<(), StorageError> {
        let _: Option<SurrealAssetRecord> = self
            .db
            .delete((TABLE_ASSET, id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl UnitRepository for SurrealStore {
    async fn get(&self, id: UnitId) -> Result<Option<Unit>, StorageError> {
        let record: Option<SurrealUnitRecord> = self
            .db
            .select((TABLE_UNIT, id.to_string()))
            .await
            .map_err(map_err)?;
        match record {
            Some(record) => Ok(Some(record.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Unit>, StorageError> {
        #[derive(Serialize)]
        struct Bindings {
            tenant_id: String,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM unit WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id: tenant_id.to_string(),
                limit,
                offset,
            })
            .await
            .map_err(map_err)?;

        let records: Vec<SurrealUnitRecord> = response.take(0).map_err(map_err)?;
        records.into_iter().map(Unit::try_from).collect()
    }

    async fn upsert(&self, unit: Unit) -> Result<(), StorageError> {
        let record = SurrealUnitWrite::from(&unit);
        let _: Option<SurrealUnitRecord> = self
            .db
            .upsert((TABLE_UNIT, unit.id.to_string()))
            .content(record)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: UnitId) -> Result<(), StorageError> {
        let _: Option<SurrealUnitRecord> = self
            .db
            .delete((TABLE_UNIT, id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl TeamRepository for SurrealStore {
    async fn get(&self, id: TeamId) -> Result<Option<Team>, StorageError> {
        let record: Option<SurrealTeamRecord> = self
            .db
            .select((TABLE_TEAM, id.to_string()))
            .await
            .map_err(map_err)?;
        match record {
            Some(record) => Ok(Some(record.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Team>, StorageError> {
        #[derive(Serialize)]
        struct Bindings {
            tenant_id: String,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM team WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id: tenant_id.to_string(),
                limit,
                offset,
            })
            .await
            .map_err(map_err)?;

        let records: Vec<SurrealTeamRecord> = response.take(0).map_err(map_err)?;
        records.into_iter().map(Team::try_from).collect()
    }

    async fn upsert(&self, team: Team) -> Result<(), StorageError> {
        let record = SurrealTeamWrite::from(&team);
        let _: Option<SurrealTeamRecord> = self
            .db
            .upsert((TABLE_TEAM, team.id.to_string()))
            .content(record)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: TeamId) -> Result<(), StorageError> {
        let _: Option<SurrealTeamRecord> = self
            .db
            .delete((TABLE_TEAM, id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl CapabilityRepository for SurrealStore {
    async fn get(&self, id: CapabilityId) -> Result<Option<Capability>, StorageError> {
        let record: Option<SurrealCapabilityRecord> = self
            .db
            .select((TABLE_CAPABILITY, id.to_string()))
            .await
            .map_err(map_err)?;
        match record {
            Some(record) => Ok(Some(record.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Capability>, StorageError> {
        #[derive(Serialize)]
        struct Bindings {
            tenant_id: String,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM capability WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id: tenant_id.to_string(),
                limit,
                offset,
            })
            .await
            .map_err(map_err)?;

        let records: Vec<SurrealCapabilityRecord> = response.take(0).map_err(map_err)?;
        records
            .into_iter()
            .map(Capability::try_from)
            .collect()
    }

    async fn upsert(&self, capability: Capability) -> Result<(), StorageError> {
        let record = SurrealCapabilityWrite::from(&capability);
        let _: Option<SurrealCapabilityRecord> = self
            .db
            .upsert((TABLE_CAPABILITY, capability.id.to_string()))
            .content(record)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: CapabilityId) -> Result<(), StorageError> {
        let _: Option<SurrealCapabilityRecord> = self
            .db
            .delete((TABLE_CAPABILITY, id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl IncidentRepository for SurrealStore {
    async fn get(&self, id: IncidentId) -> Result<Option<Incident>, StorageError> {
        let record: Option<SurrealIncidentRecord> = self
            .db
            .select((TABLE_INCIDENT, id.to_string()))
            .await
            .map_err(map_err)?;
        match record {
            Some(record) => Ok(Some(record.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Incident>, StorageError> {
        #[derive(Serialize)]
        struct Bindings {
            tenant_id: String,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM incident WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id: tenant_id.to_string(),
                limit,
                offset,
            })
            .await
            .map_err(map_err)?;

        let records: Vec<SurrealIncidentRecord> = response.take(0).map_err(map_err)?;
        records
            .into_iter()
            .map(Incident::try_from)
            .collect()
    }

    async fn upsert(&self, incident: Incident) -> Result<(), StorageError> {
        let record = SurrealIncidentWrite::from(&incident);
        let _: Option<SurrealIncidentRecord> = self
            .db
            .upsert((TABLE_INCIDENT, incident.id.to_string()))
            .content(record)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: IncidentId) -> Result<(), StorageError> {
        let _: Option<SurrealIncidentRecord> = self
            .db
            .delete((TABLE_INCIDENT, id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl TaskRepository for SurrealStore {
    async fn get(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        let record: Option<SurrealTaskRecord> = self
            .db
            .select((TABLE_TASK, id.to_string()))
            .await
            .map_err(map_err)?;
        match record {
            Some(record) => Ok(Some(record.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_by_mission(
        &self,
        mission_id: MissionId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Task>, StorageError> {
        #[derive(Serialize)]
        struct Bindings {
            mission_id: String,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM task WHERE mission_id = $mission_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                mission_id: mission_id.to_string(),
                limit,
                offset,
            })
            .await
            .map_err(map_err)?;

        let records: Vec<SurrealTaskRecord> = response.take(0).map_err(map_err)?;
        records.into_iter().map(Task::try_from).collect()
    }

    async fn upsert(&self, task: Task) -> Result<(), StorageError> {
        let record = SurrealTaskWrite::from(&task);
        let _: Option<SurrealTaskRecord> = self
            .db
            .upsert((TABLE_TASK, task.id.to_string()))
            .content(record)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: TaskId) -> Result<(), StorageError> {
        let _: Option<SurrealTaskRecord> = self
            .db
            .delete((TABLE_TASK, id.to_string()))
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

fn env_var(key: &str, default: String) -> String {
    env::var(key).unwrap_or(default)
}

fn env_var_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_var_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn map_err(err: impl std::fmt::Display) -> StorageError {
    StorageError::new(err.to_string())
}

fn thing_uuid(thing: &Thing) -> Result<Uuid, StorageError> {
    match &thing.id {
        Id::Uuid(value) => Ok((*value).into()),
        Id::String(value) => Uuid::parse_str(value).map_err(map_err),
        _ => Err(StorageError::new("unsupported SurrealDB record id type")),
    }
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::new(format!("invalid {field}")))
}

impl TryFrom<SurrealMissionRecord> for Mission {
    type Error = StorageError;

    fn try_from(value: SurrealMissionRecord) -> Result<Self, Self::Error> {
        Ok(Mission {
            id: MissionId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: TenantId::from_uuid(parse_uuid(&value.tenant_id, "tenant_id")?),
            name: value.name,
            status: value.status,
            priority: value.priority,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Mission> for SurrealMissionWrite {
    fn from(value: &Mission) -> Self {
        Self {
            tenant_id: value.tenant_id.to_string(),
            name: value.name.clone(),
            status: value.status,
            priority: value.priority,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

impl TryFrom<SurrealAssetRecord> for Asset {
    type Error = StorageError;

    fn try_from(value: SurrealAssetRecord) -> Result<Self, Self::Error> {
        let unit_id = match value.unit_id {
            Some(raw) => Some(UnitId::from_uuid(parse_uuid(&raw, "unit_id")?)),
            None => None,
        };
        let capability_ids = value
            .capability_ids
            .into_iter()
            .map(|raw| parse_uuid(&raw, "capability_id").map(CapabilityId::from_uuid))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Asset {
            id: AssetId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: TenantId::from_uuid(parse_uuid(&value.tenant_id, "tenant_id")?),
            name: value.name,
            kind: value.kind,
            status: value.status,
            readiness: value.readiness,
            comms_status: value.comms_status,
            maintenance_state: value.maintenance_state,
            unit_id,
            capability_ids,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Asset> for SurrealAssetWrite {
    fn from(value: &Asset) -> Self {
        Self {
            tenant_id: value.tenant_id.to_string(),
            name: value.name.clone(),
            kind: value.kind,
            status: value.status,
            readiness: value.readiness,
            comms_status: value.comms_status,
            maintenance_state: value.maintenance_state,
            unit_id: value.unit_id.map(|id| id.to_string()),
            capability_ids: value
                .capability_ids
                .iter()
                .map(ToString::to_string)
                .collect(),
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

impl TryFrom<SurrealUnitRecord> for Unit {
    type Error = StorageError;

    fn try_from(value: SurrealUnitRecord) -> Result<Self, Self::Error> {
        let team_id = match value.team_id {
            Some(raw) => Some(TeamId::from_uuid(parse_uuid(&raw, "team_id")?)),
            None => None,
        };
        let capability_ids = value
            .capability_ids
            .into_iter()
            .map(|raw| parse_uuid(&raw, "capability_id").map(CapabilityId::from_uuid))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Unit {
            id: UnitId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: TenantId::from_uuid(parse_uuid(&value.tenant_id, "tenant_id")?),
            classification: value.classification,
            callsign: value.callsign,
            display_name: value.display_name,
            readiness: value.readiness,
            comms_status: value.comms_status,
            team_id,
            capability_ids,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Unit> for SurrealUnitWrite {
    fn from(value: &Unit) -> Self {
        Self {
            tenant_id: value.tenant_id.to_string(),
            display_name: value.display_name.clone(),
            callsign: value.callsign.clone(),
            classification: value.classification,
            readiness: value.readiness,
            comms_status: value.comms_status,
            team_id: value.team_id.map(|id| id.to_string()),
            capability_ids: value
                .capability_ids
                .iter()
                .map(ToString::to_string)
                .collect(),
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

impl TryFrom<SurrealTeamRecord> for Team {
    type Error = StorageError;

    fn try_from(value: SurrealTeamRecord) -> Result<Self, Self::Error> {
        Ok(Team {
            id: TeamId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: TenantId::from_uuid(parse_uuid(&value.tenant_id, "tenant_id")?),
            name: value.name,
            callsign: value.callsign,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Team> for SurrealTeamWrite {
    fn from(value: &Team) -> Self {
        Self {
            tenant_id: value.tenant_id.to_string(),
            name: value.name.clone(),
            callsign: value.callsign.clone(),
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

impl TryFrom<SurrealCapabilityRecord> for Capability {
    type Error = StorageError;

    fn try_from(value: SurrealCapabilityRecord) -> Result<Self, Self::Error> {
        Ok(Capability {
            id: CapabilityId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: TenantId::from_uuid(parse_uuid(&value.tenant_id, "tenant_id")?),
            code: value.code,
            name: value.name,
            category: value.category,
            description: value.description,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Capability> for SurrealCapabilityWrite {
    fn from(value: &Capability) -> Self {
        Self {
            tenant_id: value.tenant_id.to_string(),
            code: value.code.clone(),
            name: value.name.clone(),
            category: value.category.clone(),
            description: value.description.clone(),
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

impl TryFrom<SurrealIncidentRecord> for Incident {
    type Error = StorageError;

    fn try_from(value: SurrealIncidentRecord) -> Result<Self, Self::Error> {
        Ok(Incident {
            id: IncidentId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: TenantId::from_uuid(parse_uuid(&value.tenant_id, "tenant_id")?),
            incident_type: value.incident_type,
            status: value.status,
            summary: value.summary,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Incident> for SurrealIncidentWrite {
    fn from(value: &Incident) -> Self {
        Self {
            tenant_id: value.tenant_id.to_string(),
            incident_type: value.incident_type,
            status: value.status,
            summary: value.summary.clone(),
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

impl TryFrom<SurrealTaskRecord> for Task {
    type Error = StorageError;

    fn try_from(value: SurrealTaskRecord) -> Result<Self, Self::Error> {
        Ok(Task {
            id: TaskId::from_uuid(thing_uuid(&value.id)?),
            mission_id: MissionId::from_uuid(parse_uuid(&value.mission_id, "mission_id")?),
            tenant_id: TenantId::from_uuid(parse_uuid(&value.tenant_id, "tenant_id")?),
            title: value.title,
            status: value.status,
            priority: value.priority,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Task> for SurrealTaskWrite {
    fn from(value: &Task) -> Self {
        Self {
            tenant_id: value.tenant_id.to_string(),
            mission_id: value.mission_id.to_string(),
            title: value.title.clone(),
            status: value.status,
            priority: value.priority,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}
