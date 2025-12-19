use async_trait::async_trait;
use c2_core::{Asset, AssetId, Incident, IncidentId, Mission, MissionId, TenantId};
use c2_storage::{AssetRepository, IncidentRepository, MissionRepository, StorageError};
use std::env;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

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
        Ok(Self { db })
    }
}

#[async_trait]
impl MissionRepository for SurrealStore {
    async fn get(&self, _id: MissionId) -> Result<Option<Mission>, StorageError> {
        Err(StorageError::new("mission get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Mission>, StorageError> {
        Err(StorageError::new("mission list not implemented"))
    }

    async fn upsert(&self, _mission: Mission) -> Result<(), StorageError> {
        Err(StorageError::new("mission upsert not implemented"))
    }
}

#[async_trait]
impl AssetRepository for SurrealStore {
    async fn get(&self, _id: AssetId) -> Result<Option<Asset>, StorageError> {
        Err(StorageError::new("asset get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Asset>, StorageError> {
        Err(StorageError::new("asset list not implemented"))
    }

    async fn upsert(&self, _asset: Asset) -> Result<(), StorageError> {
        Err(StorageError::new("asset upsert not implemented"))
    }
}

#[async_trait]
impl IncidentRepository for SurrealStore {
    async fn get(&self, _id: IncidentId) -> Result<Option<Incident>, StorageError> {
        Err(StorageError::new("incident get not implemented"))
    }

    async fn list_by_tenant(
        &self,
        _tenant_id: TenantId,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<Incident>, StorageError> {
        Err(StorageError::new("incident list not implemented"))
    }

    async fn upsert(&self, _incident: Incident) -> Result<(), StorageError> {
        Err(StorageError::new("incident upsert not implemented"))
    }
}

fn env_var(key: &str, default: String) -> String {
    env::var(key).unwrap_or(default)
}

fn map_err(err: impl std::fmt::Display) -> StorageError {
    StorageError::new(err.to_string())
}
