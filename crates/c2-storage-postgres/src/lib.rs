use async_trait::async_trait;
use c2_core::{
    Asset, AssetId, Capability, CapabilityId, Incident, IncidentId, Mission, MissionId, Task,
    TaskId, Team, TeamId, TenantId, Unit, UnitId,
};
use c2_storage::{
    AssetRepository, CapabilityRepository, IncidentRepository, MissionRepository, StorageError,
    TaskRepository, TeamRepository, UnitRepository,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

const TABLE_MISSIONS: &str = "missions";
const TABLE_ASSETS: &str = "assets";
const TABLE_UNITS: &str = "units";
const TABLE_TEAMS: &str = "teams";
const TABLE_CAPABILITIES: &str = "capabilities";
const TABLE_INCIDENTS: &str = "incidents";
const TABLE_TASKS: &str = "tasks";

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub connection_url: String,
    pub max_connections: u32,
}

impl PostgresConfig {
    pub fn from_env() -> Self {
        let max_connections = env::var("C2_POSTGRES_MAX_CONNECTIONS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(10);
        Self {
            connection_url: env::var("C2_POSTGRES_URL")
                .unwrap_or_else(|_| "postgres://c2:changeme@localhost:5432/c2".to_string()),
            max_connections,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect(config: &PostgresConfig) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.connection_url)
            .await
            .map_err(map_err)?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(map_err)?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl MissionRepository for PostgresStore {
    async fn get(&self, id: MissionId) -> Result<Option<Mission>, StorageError> {
        let payload: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE id = $1",
            TABLE_MISSIONS
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match payload {
            Some(value) => Ok(Some(from_json(value)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Mission>, StorageError> {
        let payloads: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE tenant_id = $1 ORDER BY created_at_ms DESC LIMIT $2 OFFSET $3",
            TABLE_MISSIONS
        ))
        .bind(tenant_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        payloads
            .into_iter()
            .map(from_json::<Mission>)
            .collect()
    }

    async fn upsert(&self, mission: Mission) -> Result<(), StorageError> {
        let payload = to_json(&mission)?;
        let status = enum_to_string(&mission.status)?;
        let priority = enum_to_string(&mission.priority)?;
        let classification = enum_to_string(&mission.classification)?;
        sqlx::query(&format!(
            "INSERT INTO {} \
             (id, tenant_id, name, status, priority, classification, created_at_ms, updated_at_ms, created_at, updated_at, payload) \
             VALUES \
             ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($7 / 1000.0), to_timestamp($8 / 1000.0), $9) \
             ON CONFLICT (id) DO UPDATE SET \
             name = EXCLUDED.name, \
             status = EXCLUDED.status, \
             priority = EXCLUDED.priority, \
             classification = EXCLUDED.classification, \
             updated_at_ms = EXCLUDED.updated_at_ms, \
             updated_at = EXCLUDED.updated_at, \
             payload = EXCLUDED.payload",
            TABLE_MISSIONS
        ))
        .bind(mission.id.as_uuid())
        .bind(mission.tenant_id.as_uuid())
        .bind(mission.name)
        .bind(status)
        .bind(priority)
        .bind(classification)
        .bind(to_i64(mission.created_at_ms)?)
        .bind(to_i64(mission.updated_at_ms)?)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: MissionId) -> Result<(), StorageError> {
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", TABLE_MISSIONS))
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl AssetRepository for PostgresStore {
    async fn get(&self, id: AssetId) -> Result<Option<Asset>, StorageError> {
        let payload: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE id = $1",
            TABLE_ASSETS
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match payload {
            Some(value) => Ok(Some(from_json(value)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Asset>, StorageError> {
        let payloads: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE tenant_id = $1 ORDER BY created_at_ms DESC LIMIT $2 OFFSET $3",
            TABLE_ASSETS
        ))
        .bind(tenant_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        payloads
            .into_iter()
            .map(from_json::<Asset>)
            .collect()
    }

    async fn upsert(&self, asset: Asset) -> Result<(), StorageError> {
        let payload = to_json(&asset)?;
        let kind = enum_to_string(&asset.kind)?;
        let status = enum_to_string(&asset.status)?;
        let classification = enum_to_string(&asset.classification)?;
        sqlx::query(&format!(
            "INSERT INTO {} \
             (id, tenant_id, name, kind, status, classification, created_at_ms, updated_at_ms, created_at, updated_at, payload) \
             VALUES \
             ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($7 / 1000.0), to_timestamp($8 / 1000.0), $9) \
             ON CONFLICT (id) DO UPDATE SET \
             name = EXCLUDED.name, \
             kind = EXCLUDED.kind, \
             status = EXCLUDED.status, \
             classification = EXCLUDED.classification, \
             updated_at_ms = EXCLUDED.updated_at_ms, \
             updated_at = EXCLUDED.updated_at, \
             payload = EXCLUDED.payload",
            TABLE_ASSETS
        ))
        .bind(asset.id.as_uuid())
        .bind(asset.tenant_id.as_uuid())
        .bind(asset.name)
        .bind(kind)
        .bind(status)
        .bind(classification)
        .bind(to_i64(asset.created_at_ms)?)
        .bind(to_i64(asset.updated_at_ms)?)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: AssetId) -> Result<(), StorageError> {
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", TABLE_ASSETS))
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl UnitRepository for PostgresStore {
    async fn get(&self, id: UnitId) -> Result<Option<Unit>, StorageError> {
        let payload: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE id = $1",
            TABLE_UNITS
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match payload {
            Some(value) => Ok(Some(from_json(value)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Unit>, StorageError> {
        let payloads: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE tenant_id = $1 ORDER BY created_at_ms DESC LIMIT $2 OFFSET $3",
            TABLE_UNITS
        ))
        .bind(tenant_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        payloads
            .into_iter()
            .map(from_json::<Unit>)
            .collect()
    }

    async fn upsert(&self, unit: Unit) -> Result<(), StorageError> {
        let payload = to_json(&unit)?;
        let readiness = enum_to_string(&unit.readiness)?;
        let comms_status = enum_to_string(&unit.comms_status)?;
        let classification = enum_to_string(&unit.classification)?;
        sqlx::query(&format!(
            "INSERT INTO {} \
             (id, tenant_id, display_name, callsign, readiness, comms_status, classification, created_at_ms, updated_at_ms, created_at, updated_at, payload) \
             VALUES \
             ($1, $2, $3, $4, $5, $6, $7, $8, $9, to_timestamp($8 / 1000.0), to_timestamp($9 / 1000.0), $10) \
             ON CONFLICT (id) DO UPDATE SET \
             display_name = EXCLUDED.display_name, \
             callsign = EXCLUDED.callsign, \
             readiness = EXCLUDED.readiness, \
             comms_status = EXCLUDED.comms_status, \
             classification = EXCLUDED.classification, \
             updated_at_ms = EXCLUDED.updated_at_ms, \
             updated_at = EXCLUDED.updated_at, \
             payload = EXCLUDED.payload",
            TABLE_UNITS
        ))
        .bind(unit.id.as_uuid())
        .bind(unit.tenant_id.as_uuid())
        .bind(unit.display_name)
        .bind(unit.callsign)
        .bind(readiness)
        .bind(comms_status)
        .bind(classification)
        .bind(to_i64(unit.created_at_ms)?)
        .bind(to_i64(unit.updated_at_ms)?)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: UnitId) -> Result<(), StorageError> {
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", TABLE_UNITS))
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl TeamRepository for PostgresStore {
    async fn get(&self, id: TeamId) -> Result<Option<Team>, StorageError> {
        let payload: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE id = $1",
            TABLE_TEAMS
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match payload {
            Some(value) => Ok(Some(from_json(value)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Team>, StorageError> {
        let payloads: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE tenant_id = $1 ORDER BY created_at_ms DESC LIMIT $2 OFFSET $3",
            TABLE_TEAMS
        ))
        .bind(tenant_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        payloads
            .into_iter()
            .map(from_json::<Team>)
            .collect()
    }

    async fn upsert(&self, team: Team) -> Result<(), StorageError> {
        let payload = to_json(&team)?;
        let classification = enum_to_string(&team.classification)?;
        sqlx::query(&format!(
            "INSERT INTO {} \
             (id, tenant_id, name, callsign, classification, created_at_ms, updated_at_ms, created_at, updated_at, payload) \
             VALUES \
             ($1, $2, $3, $4, $5, $6, $7, to_timestamp($6 / 1000.0), to_timestamp($7 / 1000.0), $8) \
             ON CONFLICT (id) DO UPDATE SET \
             name = EXCLUDED.name, \
             callsign = EXCLUDED.callsign, \
             classification = EXCLUDED.classification, \
             updated_at_ms = EXCLUDED.updated_at_ms, \
             updated_at = EXCLUDED.updated_at, \
             payload = EXCLUDED.payload",
            TABLE_TEAMS
        ))
        .bind(team.id.as_uuid())
        .bind(team.tenant_id.as_uuid())
        .bind(team.name)
        .bind(team.callsign)
        .bind(classification)
        .bind(to_i64(team.created_at_ms)?)
        .bind(to_i64(team.updated_at_ms)?)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: TeamId) -> Result<(), StorageError> {
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", TABLE_TEAMS))
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl CapabilityRepository for PostgresStore {
    async fn get(&self, id: CapabilityId) -> Result<Option<Capability>, StorageError> {
        let payload: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE id = $1",
            TABLE_CAPABILITIES
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match payload {
            Some(value) => Ok(Some(from_json(value)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Capability>, StorageError> {
        let payloads: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE tenant_id = $1 ORDER BY created_at_ms DESC LIMIT $2 OFFSET $3",
            TABLE_CAPABILITIES
        ))
        .bind(tenant_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        payloads
            .into_iter()
            .map(from_json::<Capability>)
            .collect()
    }

    async fn upsert(&self, capability: Capability) -> Result<(), StorageError> {
        let payload = to_json(&capability)?;
        let classification = enum_to_string(&capability.classification)?;
        sqlx::query(&format!(
            "INSERT INTO {} \
             (id, tenant_id, code, name, classification, created_at_ms, updated_at_ms, created_at, updated_at, payload) \
             VALUES \
             ($1, $2, $3, $4, $5, $6, $7, to_timestamp($6 / 1000.0), to_timestamp($7 / 1000.0), $8) \
             ON CONFLICT (id) DO UPDATE SET \
             code = EXCLUDED.code, \
             name = EXCLUDED.name, \
             classification = EXCLUDED.classification, \
             updated_at_ms = EXCLUDED.updated_at_ms, \
             updated_at = EXCLUDED.updated_at, \
             payload = EXCLUDED.payload",
            TABLE_CAPABILITIES
        ))
        .bind(capability.id.as_uuid())
        .bind(capability.tenant_id.as_uuid())
        .bind(capability.code)
        .bind(capability.name)
        .bind(classification)
        .bind(to_i64(capability.created_at_ms)?)
        .bind(to_i64(capability.updated_at_ms)?)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: CapabilityId) -> Result<(), StorageError> {
        sqlx::query(&format!(
            "DELETE FROM {} WHERE id = $1",
            TABLE_CAPABILITIES
        ))
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl IncidentRepository for PostgresStore {
    async fn get(&self, id: IncidentId) -> Result<Option<Incident>, StorageError> {
        let payload: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE id = $1",
            TABLE_INCIDENTS
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match payload {
            Some(value) => Ok(Some(from_json(value)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Incident>, StorageError> {
        let payloads: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE tenant_id = $1 ORDER BY created_at_ms DESC LIMIT $2 OFFSET $3",
            TABLE_INCIDENTS
        ))
        .bind(tenant_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        payloads
            .into_iter()
            .map(from_json::<Incident>)
            .collect()
    }

    async fn upsert(&self, incident: Incident) -> Result<(), StorageError> {
        let payload = to_json(&incident)?;
        let incident_type = enum_to_string(&incident.incident_type)?;
        let status = enum_to_string(&incident.status)?;
        let classification = enum_to_string(&incident.classification)?;
        sqlx::query(&format!(
            "INSERT INTO {} \
             (id, tenant_id, incident_type, status, summary, classification, created_at_ms, updated_at_ms, created_at, updated_at, payload) \
             VALUES \
             ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($7 / 1000.0), to_timestamp($8 / 1000.0), $9) \
             ON CONFLICT (id) DO UPDATE SET \
             incident_type = EXCLUDED.incident_type, \
             status = EXCLUDED.status, \
             summary = EXCLUDED.summary, \
             classification = EXCLUDED.classification, \
             updated_at_ms = EXCLUDED.updated_at_ms, \
             updated_at = EXCLUDED.updated_at, \
             payload = EXCLUDED.payload",
            TABLE_INCIDENTS
        ))
        .bind(incident.id.as_uuid())
        .bind(incident.tenant_id.as_uuid())
        .bind(incident_type)
        .bind(status)
        .bind(incident.summary)
        .bind(classification)
        .bind(to_i64(incident.created_at_ms)?)
        .bind(to_i64(incident.updated_at_ms)?)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: IncidentId) -> Result<(), StorageError> {
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", TABLE_INCIDENTS))
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

#[async_trait]
impl TaskRepository for PostgresStore {
    async fn get(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        let payload: Option<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE id = $1",
            TABLE_TASKS
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        match payload {
            Some(value) => Ok(Some(from_json(value)?)),
            None => Ok(None),
        }
    }

    async fn list_by_mission(
        &self,
        mission_id: MissionId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Task>, StorageError> {
        let payloads: Vec<Value> = sqlx::query_scalar(&format!(
            "SELECT payload FROM {} WHERE mission_id = $1 ORDER BY created_at_ms DESC LIMIT $2 OFFSET $3",
            TABLE_TASKS
        ))
        .bind(mission_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        payloads
            .into_iter()
            .map(from_json::<Task>)
            .collect()
    }

    async fn upsert(&self, task: Task) -> Result<(), StorageError> {
        let payload = to_json(&task)?;
        let status = enum_to_string(&task.status)?;
        let priority = enum_to_string(&task.priority)?;
        let classification = enum_to_string(&task.classification)?;
        sqlx::query(&format!(
            "INSERT INTO {} \
             (id, mission_id, tenant_id, title, status, priority, classification, created_at_ms, updated_at_ms, created_at, updated_at, payload) \
             VALUES \
             ($1, $2, $3, $4, $5, $6, $7, $8, $9, to_timestamp($8 / 1000.0), to_timestamp($9 / 1000.0), $10) \
             ON CONFLICT (id) DO UPDATE SET \
             title = EXCLUDED.title, \
             status = EXCLUDED.status, \
             priority = EXCLUDED.priority, \
             classification = EXCLUDED.classification, \
             updated_at_ms = EXCLUDED.updated_at_ms, \
             updated_at = EXCLUDED.updated_at, \
             payload = EXCLUDED.payload",
            TABLE_TASKS
        ))
        .bind(task.id.as_uuid())
        .bind(task.mission_id.as_uuid())
        .bind(task.tenant_id.as_uuid())
        .bind(task.title)
        .bind(status)
        .bind(priority)
        .bind(classification)
        .bind(to_i64(task.created_at_ms)?)
        .bind(to_i64(task.updated_at_ms)?)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: TaskId) -> Result<(), StorageError> {
        sqlx::query(&format!("DELETE FROM {} WHERE id = $1", TABLE_TASKS))
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

fn to_json<T: Serialize>(value: &T) -> Result<Value, StorageError> {
    serde_json::to_value(value).map_err(map_err)
}

fn from_json<T: DeserializeOwned>(value: Value) -> Result<T, StorageError> {
    serde_json::from_value(value).map_err(map_err)
}

fn enum_to_string<T: Serialize>(value: &T) -> Result<String, StorageError> {
    match serde_json::to_value(value).map_err(map_err)? {
        Value::String(value) => Ok(value),
        _ => Err(StorageError::new("expected enum string value")),
    }
}

fn to_i64(value: u64) -> Result<i64, StorageError> {
    i64::try_from(value).map_err(|_| StorageError::new("timestamp overflow"))
}

fn map_err(err: impl std::fmt::Display) -> StorageError {
    StorageError::new(err.to_string())
}
