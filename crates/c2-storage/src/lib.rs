use async_trait::async_trait;
use c2_core::{Asset, AssetId, Incident, IncidentId, Mission, MissionId, TenantId};
use std::fmt;

#[derive(Debug, Clone)]
pub struct StorageError {
    pub message: String,
}

impl StorageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StorageError {}

#[async_trait]
pub trait MissionRepository: Send + Sync {
    async fn get(&self, id: MissionId) -> Result<Option<Mission>, StorageError>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Mission>, StorageError>;
    async fn upsert(&self, mission: Mission) -> Result<(), StorageError>;
}

#[async_trait]
pub trait AssetRepository: Send + Sync {
    async fn get(&self, id: AssetId) -> Result<Option<Asset>, StorageError>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Asset>, StorageError>;
    async fn upsert(&self, asset: Asset) -> Result<(), StorageError>;
}

#[async_trait]
pub trait IncidentRepository: Send + Sync {
    async fn get(&self, id: IncidentId) -> Result<Option<Incident>, StorageError>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Incident>, StorageError>;
    async fn upsert(&self, incident: Incident) -> Result<(), StorageError>;
}
