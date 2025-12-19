use async_trait::async_trait;
use c2_core::{Asset, AssetId, Incident, IncidentId, Mission, MissionId, TenantId};
use c2_storage::{AssetRepository, IncidentRepository, MissionRepository, StorageError};
use std::env;

#[derive(Debug, Clone)]
pub struct TimescaleConfig {
    pub connection_url: String,
}

impl TimescaleConfig {
    pub fn from_env() -> Self {
        Self {
            connection_url: env::var("C2_TIMESCALE_URL")
                .unwrap_or_else(|_| "postgres://c2:changeme@localhost:5432/c2".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimescaleStore {
    pub config: TimescaleConfig,
}

impl TimescaleStore {
    pub fn new(config: TimescaleConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MissionRepository for TimescaleStore {
    async fn get(&self, _id: MissionId) -> Result<Option<Mission>, StorageError> {
        Err(StorageError::new("timescale mission get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Mission>, StorageError> {
        Err(StorageError::new("timescale mission list not implemented"))
    }

    async fn upsert(&self, _mission: Mission) -> Result<(), StorageError> {
        Err(StorageError::new("timescale mission upsert not implemented"))
    }
}

#[async_trait]
impl AssetRepository for TimescaleStore {
    async fn get(&self, _id: AssetId) -> Result<Option<Asset>, StorageError> {
        Err(StorageError::new("timescale asset get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Asset>, StorageError> {
        Err(StorageError::new("timescale asset list not implemented"))
    }

    async fn upsert(&self, _asset: Asset) -> Result<(), StorageError> {
        Err(StorageError::new("timescale asset upsert not implemented"))
    }
}

#[async_trait]
impl IncidentRepository for TimescaleStore {
    async fn get(&self, _id: IncidentId) -> Result<Option<Incident>, StorageError> {
        Err(StorageError::new("timescale incident get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Incident>, StorageError> {
        Err(StorageError::new("timescale incident list not implemented"))
    }

    async fn upsert(&self, _incident: Incident) -> Result<(), StorageError> {
        Err(StorageError::new("timescale incident upsert not implemented"))
    }
}
