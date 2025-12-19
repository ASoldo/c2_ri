use async_trait::async_trait;
use c2_core::{
    Asset, AssetId, AssetKind, AssetStatus, Incident, IncidentId, IncidentStatus, IncidentType,
    Mission, MissionId, MissionStatus, OperationalPriority, SecurityClassification, Task, TaskId,
    TaskStatus, TenantId,
};
use c2_storage::{
    AssetRepository, IncidentRepository, MissionRepository, StorageError, TaskRepository,
};
use serde::{Deserialize, Serialize};
use std::env;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::sql::{Id, Thing};
use surrealdb::Surreal;
use uuid::Uuid;

const TABLE_MISSION: &str = "mission";
const TABLE_ASSET: &str = "asset";
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
}

impl SurrealConfig {
    pub fn from_env() -> Self {
        Self {
            endpoint: env_var("C2_SURREAL_ENDPOINT", "ws://127.0.0.1:8000".to_string()),
            namespace: env_var("C2_SURREAL_NAMESPACE", "c2".to_string()),
            database: env_var("C2_SURREAL_DATABASE", "operations".to_string()),
            username: env_var("C2_SURREAL_USERNAME", "root".to_string()),
            password: env_var("C2_SURREAL_PASSWORD", "root".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SurrealStore {
    #[allow(dead_code)]
    db: Surreal<Client>,
}

#[derive(Debug, Deserialize)]
struct SurrealMissionRecord {
    id: Thing,
    tenant_id: TenantId,
    name: String,
    status: MissionStatus,
    priority: OperationalPriority,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealMissionWrite {
    tenant_id: TenantId,
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
    tenant_id: TenantId,
    name: String,
    kind: AssetKind,
    status: AssetStatus,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealAssetWrite {
    tenant_id: TenantId,
    name: String,
    kind: AssetKind,
    status: AssetStatus,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SurrealIncidentRecord {
    id: Thing,
    tenant_id: TenantId,
    incident_type: IncidentType,
    status: IncidentStatus,
    summary: String,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealIncidentWrite {
    tenant_id: TenantId,
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
    tenant_id: TenantId,
    mission_id: MissionId,
    title: String,
    status: TaskStatus,
    priority: OperationalPriority,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Serialize)]
struct SurrealTaskWrite {
    tenant_id: TenantId,
    mission_id: MissionId,
    title: String,
    status: TaskStatus,
    priority: OperationalPriority,
    classification: SecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl SurrealStore {
    pub async fn connect(config: &SurrealConfig) -> Result<Self, StorageError> {
        let db = Surreal::new::<Ws>(&config.endpoint)
            .await
            .map_err(map_err)?;
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
            tenant_id: TenantId,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM mission WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id,
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
            tenant_id: TenantId,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM asset WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id,
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
            tenant_id: TenantId,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM incident WHERE tenant_id = $tenant_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                tenant_id,
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
            mission_id: MissionId,
            limit: usize,
            offset: usize,
        }

        let mut response = self
            .db
            .query(
                "SELECT * FROM task WHERE mission_id = $mission_id ORDER BY created_at_ms DESC LIMIT $limit START $offset",
            )
            .bind(Bindings {
                mission_id,
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

impl TryFrom<SurrealMissionRecord> for Mission {
    type Error = StorageError;

    fn try_from(value: SurrealMissionRecord) -> Result<Self, Self::Error> {
        Ok(Mission {
            id: MissionId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: value.tenant_id,
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
            tenant_id: value.tenant_id,
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
        Ok(Asset {
            id: AssetId::from_uuid(thing_uuid(&value.id)?),
            tenant_id: value.tenant_id,
            name: value.name,
            kind: value.kind,
            status: value.status,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        })
    }
}

impl From<&Asset> for SurrealAssetWrite {
    fn from(value: &Asset) -> Self {
        Self {
            tenant_id: value.tenant_id,
            name: value.name.clone(),
            kind: value.kind,
            status: value.status,
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
            tenant_id: value.tenant_id,
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
            tenant_id: value.tenant_id,
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
            mission_id: value.mission_id,
            tenant_id: value.tenant_id,
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
            tenant_id: value.tenant_id,
            mission_id: value.mission_id,
            title: value.title.clone(),
            status: value.status,
            priority: value.priority,
            classification: value.classification,
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}
