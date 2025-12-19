use async_trait::async_trait;
use c2_core::{Asset, AssetId, Incident, IncidentId, Mission, MissionId, TenantId};
use c2_storage::{AssetRepository, IncidentRepository, MissionRepository, StorageError};
use std::env;

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub connection_url: String,
}

impl PostgresConfig {
    pub fn from_env() -> Self {
        Self {
            connection_url: env::var("C2_POSTGRES_URL")
                .unwrap_or_else(|_| "postgres://c2:changeme@localhost:5432/c2".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PostgresStore {
    pub config: PostgresConfig,
}

impl PostgresStore {
    pub fn new(config: PostgresConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MissionRepository for PostgresStore {
    async fn get(&self, _id: MissionId) -> Result<Option<Mission>, StorageError> {
        Err(StorageError::new("postgres mission get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Mission>, StorageError> {
        Err(StorageError::new("postgres mission list not implemented"))
    }

    async fn upsert(&self, _mission: Mission) -> Result<(), StorageError> {
        Err(StorageError::new("postgres mission upsert not implemented"))
    }
}

#[async_trait]
impl AssetRepository for PostgresStore {
    async fn get(&self, _id: AssetId) -> Result<Option<Asset>, StorageError> {
        Err(StorageError::new("postgres asset get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Asset>, StorageError> {
        Err(StorageError::new("postgres asset list not implemented"))
    }

    async fn upsert(&self, _asset: Asset) -> Result<(), StorageError> {
        Err(StorageError::new("postgres asset upsert not implemented"))
    }
}

#[async_trait]
impl IncidentRepository for PostgresStore {
    async fn get(&self, _id: IncidentId) -> Result<Option<Incident>, StorageError> {
        Err(StorageError::new("postgres incident get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Incident>, StorageError> {
        Err(StorageError::new("postgres incident list not implemented"))
    }

    async fn upsert(&self, _incident: Incident) -> Result<(), StorageError> {
        Err(StorageError::new("postgres incident upsert not implemented"))
    }
}
