use async_trait::async_trait;
use c2_core::{
    Asset, AssetId, Capability, CapabilityId, Incident, IncidentId, Mission, MissionId, Task,
    TaskId, Team, TeamId, TenantId, Unit, UnitId,
};
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
    async fn delete(&self, id: MissionId) -> Result<(), StorageError>;
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
    async fn delete(&self, id: AssetId) -> Result<(), StorageError>;
}

#[async_trait]
pub trait UnitRepository: Send + Sync {
    async fn get(&self, id: UnitId) -> Result<Option<Unit>, StorageError>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Unit>, StorageError>;
    async fn upsert(&self, unit: Unit) -> Result<(), StorageError>;
    async fn delete(&self, id: UnitId) -> Result<(), StorageError>;
}

#[async_trait]
pub trait TeamRepository: Send + Sync {
    async fn get(&self, id: TeamId) -> Result<Option<Team>, StorageError>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Team>, StorageError>;
    async fn upsert(&self, team: Team) -> Result<(), StorageError>;
    async fn delete(&self, id: TeamId) -> Result<(), StorageError>;
}

#[async_trait]
pub trait CapabilityRepository: Send + Sync {
    async fn get(&self, id: CapabilityId) -> Result<Option<Capability>, StorageError>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Capability>, StorageError>;
    async fn upsert(&self, capability: Capability) -> Result<(), StorageError>;
    async fn delete(&self, id: CapabilityId) -> Result<(), StorageError>;
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
    async fn delete(&self, id: IncidentId) -> Result<(), StorageError>;
}

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn get(&self, id: TaskId) -> Result<Option<Task>, StorageError>;
    async fn list_by_mission(
        &self,
        mission_id: MissionId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Task>, StorageError>;
    async fn upsert(&self, task: Task) -> Result<(), StorageError>;
    async fn delete(&self, id: TaskId) -> Result<(), StorageError>;
}
