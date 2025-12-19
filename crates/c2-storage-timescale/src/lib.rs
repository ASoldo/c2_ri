use async_trait::async_trait;
use c2_core::{
    Asset, AssetId, Incident, IncidentId, Mission, MissionId, Task, TaskId, TenantId,
};
use c2_storage::{
    AssetRepository, IncidentRepository, MissionRepository, StorageError, TaskRepository,
};
use c2_storage_postgres::{PostgresConfig, PostgresStore};
use sqlx::PgPool;
use std::env;

#[derive(Debug, Clone)]
pub struct TimescaleConfig {
    pub connection_url: String,
    pub max_connections: u32,
}

impl TimescaleConfig {
    pub fn from_env() -> Self {
        let max_connections = env::var("C2_TIMESCALE_MAX_CONNECTIONS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(10);
        Self {
            connection_url: env::var("C2_TIMESCALE_URL")
                .unwrap_or_else(|_| "postgres://c2:changeme@localhost:5432/c2".to_string()),
            max_connections,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimescaleStore {
    inner: PostgresStore,
}

impl TimescaleStore {
    pub async fn connect(config: &TimescaleConfig) -> Result<Self, StorageError> {
        let pg_config = PostgresConfig {
            connection_url: config.connection_url.clone(),
            max_connections: config.max_connections,
        };
        let inner = PostgresStore::connect(&pg_config).await?;
        init_timescale(inner.pool()).await?;
        Ok(Self { inner })
    }

    pub fn inner(&self) -> &PostgresStore {
        &self.inner
    }
}

async fn init_timescale(pool: &PgPool) -> Result<(), StorageError> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS timescaledb")
        .execute(pool)
        .await
        .map_err(map_err)?;
    for table in ["missions", "assets", "incidents", "tasks"] {
        let statement = format!(
            "SELECT create_hypertable('{}', 'created_at', if_not_exists => TRUE)",
            table
        );
        sqlx::query(&statement).execute(pool).await.map_err(map_err)?;
    }
    Ok(())
}

fn map_err(err: impl std::fmt::Display) -> StorageError {
    StorageError::new(err.to_string())
}

#[async_trait]
impl MissionRepository for TimescaleStore {
    async fn get(&self, id: MissionId) -> Result<Option<Mission>, StorageError> {
        MissionRepository::get(&self.inner, id).await
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Mission>, StorageError> {
        MissionRepository::list_by_tenant(&self.inner, tenant_id, limit, offset).await
    }

    async fn upsert(&self, mission: Mission) -> Result<(), StorageError> {
        MissionRepository::upsert(&self.inner, mission).await
    }

    async fn delete(&self, id: MissionId) -> Result<(), StorageError> {
        MissionRepository::delete(&self.inner, id).await
    }
}

#[async_trait]
impl AssetRepository for TimescaleStore {
    async fn get(&self, id: AssetId) -> Result<Option<Asset>, StorageError> {
        AssetRepository::get(&self.inner, id).await
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Asset>, StorageError> {
        AssetRepository::list_by_tenant(&self.inner, tenant_id, limit, offset).await
    }

    async fn upsert(&self, asset: Asset) -> Result<(), StorageError> {
        AssetRepository::upsert(&self.inner, asset).await
    }

    async fn delete(&self, id: AssetId) -> Result<(), StorageError> {
        AssetRepository::delete(&self.inner, id).await
    }
}

#[async_trait]
impl IncidentRepository for TimescaleStore {
    async fn get(&self, id: IncidentId) -> Result<Option<Incident>, StorageError> {
        IncidentRepository::get(&self.inner, id).await
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Incident>, StorageError> {
        IncidentRepository::list_by_tenant(&self.inner, tenant_id, limit, offset).await
    }

    async fn upsert(&self, incident: Incident) -> Result<(), StorageError> {
        IncidentRepository::upsert(&self.inner, incident).await
    }

    async fn delete(&self, id: IncidentId) -> Result<(), StorageError> {
        IncidentRepository::delete(&self.inner, id).await
    }
}

#[async_trait]
impl TaskRepository for TimescaleStore {
    async fn get(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        TaskRepository::get(&self.inner, id).await
    }

    async fn list_by_mission(
        &self,
        mission_id: MissionId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Task>, StorageError> {
        TaskRepository::list_by_mission(&self.inner, mission_id, limit, offset).await
    }

    async fn upsert(&self, task: Task) -> Result<(), StorageError> {
        TaskRepository::upsert(&self.inner, task).await
    }

    async fn delete(&self, id: TaskId) -> Result<(), StorageError> {
        TaskRepository::delete(&self.inner, id).await
    }
}
