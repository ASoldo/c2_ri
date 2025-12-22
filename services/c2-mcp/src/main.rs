use c2_config::ServiceConfig;
use c2_core::{
    Asset, AssetId, CommsStatus, Incident, IncidentId, MaintenanceState, Mission, MissionId,
    OperationalPriority, ReadinessState, SecurityClassification, Task, TaskId, TenantId,
    now_epoch_millis,
};
use c2_identity::{Permission, Role, Subject};
use c2_observability::{init, log_startup, ObservabilityConfig};
use c2_policy::{BasicPolicyEngine, PolicyContext, PolicyDecision, PolicyEngine, PolicyRequest, ResourceDescriptor};
use axum::{routing::any_service, Router};
use c2_storage::{AssetRepository, IncidentRepository, MissionRepository, StorageError, TaskRepository};
use c2_storage_surreal::{SurrealConfig, SurrealStore};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    Annotated, ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParam,
    Meta, RawResource, RawResourceTemplate, ReadResourceRequestParam, ReadResourceResult, Resource,
    ResourceContents, ResourceTemplate, ServerCapabilities, ServerInfo,
};
use rmcp::transport::{
    streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
    StreamableHttpService,
};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use uuid::Uuid;

#[derive(Clone)]
struct C2McpService {
    store: Arc<SurrealStore>,
    policy: BasicPolicyEngine,
    default_auth: Option<AuthorizedContext>,
    tool_router: ToolRouter<Self>,
}

impl C2McpService {
    fn new(store: SurrealStore, policy: BasicPolicyEngine) -> Self {
        let store = Arc::new(store);
        let default_auth = load_default_auth();
        Self {
            store,
            policy,
            default_auth,
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpOperationalPriority {
    Routine,
    Elevated,
    Urgent,
    Critical,
}

impl From<McpOperationalPriority> for OperationalPriority {
    fn from(value: McpOperationalPriority) -> Self {
        match value {
            McpOperationalPriority::Routine => Self::Routine,
            McpOperationalPriority::Elevated => Self::Elevated,
            McpOperationalPriority::Urgent => Self::Urgent,
            McpOperationalPriority::Critical => Self::Critical,
        }
    }
}

impl From<OperationalPriority> for McpOperationalPriority {
    fn from(value: OperationalPriority) -> Self {
        match value {
            OperationalPriority::Routine => Self::Routine,
            OperationalPriority::Elevated => Self::Elevated,
            OperationalPriority::Urgent => Self::Urgent,
            OperationalPriority::Critical => Self::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpMissionStatus {
    Planned,
    Active,
    Suspended,
    Completed,
    Aborted,
}

impl From<McpMissionStatus> for c2_core::MissionStatus {
    fn from(value: McpMissionStatus) -> Self {
        match value {
            McpMissionStatus::Planned => Self::Planned,
            McpMissionStatus::Active => Self::Active,
            McpMissionStatus::Suspended => Self::Suspended,
            McpMissionStatus::Completed => Self::Completed,
            McpMissionStatus::Aborted => Self::Aborted,
        }
    }
}

impl From<c2_core::MissionStatus> for McpMissionStatus {
    fn from(value: c2_core::MissionStatus) -> Self {
        match value {
            c2_core::MissionStatus::Planned => Self::Planned,
            c2_core::MissionStatus::Active => Self::Active,
            c2_core::MissionStatus::Suspended => Self::Suspended,
            c2_core::MissionStatus::Completed => Self::Completed,
            c2_core::MissionStatus::Aborted => Self::Aborted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpTaskStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
}

impl From<McpTaskStatus> for c2_core::TaskStatus {
    fn from(value: McpTaskStatus) -> Self {
        match value {
            McpTaskStatus::Pending => Self::Pending,
            McpTaskStatus::InProgress => Self::InProgress,
            McpTaskStatus::Blocked => Self::Blocked,
            McpTaskStatus::Completed => Self::Completed,
            McpTaskStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<c2_core::TaskStatus> for McpTaskStatus {
    fn from(value: c2_core::TaskStatus) -> Self {
        match value {
            c2_core::TaskStatus::Pending => Self::Pending,
            c2_core::TaskStatus::InProgress => Self::InProgress,
            c2_core::TaskStatus::Blocked => Self::Blocked,
            c2_core::TaskStatus::Completed => Self::Completed,
            c2_core::TaskStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpAssetStatus {
    Available,
    Assigned,
    Degraded,
    Maintenance,
    Lost,
}

impl From<McpAssetStatus> for c2_core::AssetStatus {
    fn from(value: McpAssetStatus) -> Self {
        match value {
            McpAssetStatus::Available => Self::Available,
            McpAssetStatus::Assigned => Self::Assigned,
            McpAssetStatus::Degraded => Self::Degraded,
            McpAssetStatus::Maintenance => Self::Maintenance,
            McpAssetStatus::Lost => Self::Lost,
        }
    }
}

impl From<c2_core::AssetStatus> for McpAssetStatus {
    fn from(value: c2_core::AssetStatus) -> Self {
        match value {
            c2_core::AssetStatus::Available => Self::Available,
            c2_core::AssetStatus::Assigned => Self::Assigned,
            c2_core::AssetStatus::Degraded => Self::Degraded,
            c2_core::AssetStatus::Maintenance => Self::Maintenance,
            c2_core::AssetStatus::Lost => Self::Lost,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpAssetKind {
    Personnel,
    Drone,
    Ugv,
    Vehicle,
    Aircraft,
    Sensor,
    CommsRelay,
    CommandPost,
    Other,
}

impl From<McpAssetKind> for c2_core::AssetKind {
    fn from(value: McpAssetKind) -> Self {
        match value {
            McpAssetKind::Personnel => Self::Personnel,
            McpAssetKind::Drone => Self::Drone,
            McpAssetKind::Ugv => Self::Ugv,
            McpAssetKind::Vehicle => Self::Vehicle,
            McpAssetKind::Aircraft => Self::Aircraft,
            McpAssetKind::Sensor => Self::Sensor,
            McpAssetKind::CommsRelay => Self::CommsRelay,
            McpAssetKind::CommandPost => Self::CommandPost,
            McpAssetKind::Other => Self::Other,
        }
    }
}

impl From<c2_core::AssetKind> for McpAssetKind {
    fn from(value: c2_core::AssetKind) -> Self {
        match value {
            c2_core::AssetKind::Personnel => Self::Personnel,
            c2_core::AssetKind::Drone => Self::Drone,
            c2_core::AssetKind::Ugv => Self::Ugv,
            c2_core::AssetKind::Vehicle => Self::Vehicle,
            c2_core::AssetKind::Aircraft => Self::Aircraft,
            c2_core::AssetKind::Sensor => Self::Sensor,
            c2_core::AssetKind::CommsRelay => Self::CommsRelay,
            c2_core::AssetKind::CommandPost => Self::CommandPost,
            c2_core::AssetKind::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpIncidentType {
    Defense,
    Fire,
    Medical,
    Hazmat,
    Rescue,
    PublicSafety,
    Infrastructure,
    Other,
}

impl From<McpIncidentType> for c2_core::IncidentType {
    fn from(value: McpIncidentType) -> Self {
        match value {
            McpIncidentType::Defense => Self::Defense,
            McpIncidentType::Fire => Self::Fire,
            McpIncidentType::Medical => Self::Medical,
            McpIncidentType::Hazmat => Self::Hazmat,
            McpIncidentType::Rescue => Self::Rescue,
            McpIncidentType::PublicSafety => Self::PublicSafety,
            McpIncidentType::Infrastructure => Self::Infrastructure,
            McpIncidentType::Other => Self::Other,
        }
    }
}

impl From<c2_core::IncidentType> for McpIncidentType {
    fn from(value: c2_core::IncidentType) -> Self {
        match value {
            c2_core::IncidentType::Defense => Self::Defense,
            c2_core::IncidentType::Fire => Self::Fire,
            c2_core::IncidentType::Medical => Self::Medical,
            c2_core::IncidentType::Hazmat => Self::Hazmat,
            c2_core::IncidentType::Rescue => Self::Rescue,
            c2_core::IncidentType::PublicSafety => Self::PublicSafety,
            c2_core::IncidentType::Infrastructure => Self::Infrastructure,
            c2_core::IncidentType::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpIncidentStatus {
    Reported,
    Verified,
    Responding,
    Contained,
    Resolved,
    Closed,
}

impl From<McpIncidentStatus> for c2_core::IncidentStatus {
    fn from(value: McpIncidentStatus) -> Self {
        match value {
            McpIncidentStatus::Reported => Self::Reported,
            McpIncidentStatus::Verified => Self::Verified,
            McpIncidentStatus::Responding => Self::Responding,
            McpIncidentStatus::Contained => Self::Contained,
            McpIncidentStatus::Resolved => Self::Resolved,
            McpIncidentStatus::Closed => Self::Closed,
        }
    }
}

impl From<c2_core::IncidentStatus> for McpIncidentStatus {
    fn from(value: c2_core::IncidentStatus) -> Self {
        match value {
            c2_core::IncidentStatus::Reported => Self::Reported,
            c2_core::IncidentStatus::Verified => Self::Verified,
            c2_core::IncidentStatus::Responding => Self::Responding,
            c2_core::IncidentStatus::Contained => Self::Contained,
            c2_core::IncidentStatus::Resolved => Self::Resolved,
            c2_core::IncidentStatus::Closed => Self::Closed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum McpSecurityClassification {
    Unclassified,
    Controlled,
    Restricted,
    Confidential,
    Secret,
    TopSecret,
}

impl From<McpSecurityClassification> for SecurityClassification {
    fn from(value: McpSecurityClassification) -> Self {
        match value {
            McpSecurityClassification::Unclassified => Self::Unclassified,
            McpSecurityClassification::Controlled => Self::Controlled,
            McpSecurityClassification::Restricted => Self::Restricted,
            McpSecurityClassification::Confidential => Self::Confidential,
            McpSecurityClassification::Secret => Self::Secret,
            McpSecurityClassification::TopSecret => Self::TopSecret,
        }
    }
}

impl From<SecurityClassification> for McpSecurityClassification {
    fn from(value: SecurityClassification) -> Self {
        match value {
            SecurityClassification::Unclassified => Self::Unclassified,
            SecurityClassification::Controlled => Self::Controlled,
            SecurityClassification::Restricted => Self::Restricted,
            SecurityClassification::Confidential => Self::Confidential,
            SecurityClassification::Secret => Self::Secret,
            SecurityClassification::TopSecret => Self::TopSecret,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct McpAuthContext {
    tenant_id: String,
    user_id: String,
    roles: Vec<String>,
    permissions: Vec<String>,
    clearance: Option<McpSecurityClassification>,
}

#[derive(Debug, Clone)]
struct AuthorizedContext {
    subject: Subject,
    permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListMissionsParams {
    auth: Option<McpAuthContext>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GetByIdParams {
    auth: Option<McpAuthContext>,
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MissionInput {
    id: Option<String>,
    name: String,
    status: McpMissionStatus,
    priority: McpOperationalPriority,
    classification: McpSecurityClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpsertMissionParams {
    auth: Option<McpAuthContext>,
    mission: MissionInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListAssetsParams {
    auth: Option<McpAuthContext>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AssetInput {
    id: Option<String>,
    name: String,
    kind: McpAssetKind,
    status: McpAssetStatus,
    classification: McpSecurityClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpsertAssetParams {
    auth: Option<McpAuthContext>,
    asset: AssetInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListIncidentsParams {
    auth: Option<McpAuthContext>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IncidentInput {
    id: Option<String>,
    incident_type: McpIncidentType,
    status: McpIncidentStatus,
    summary: String,
    classification: McpSecurityClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpsertIncidentParams {
    auth: Option<McpAuthContext>,
    incident: IncidentInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListTasksParams {
    auth: Option<McpAuthContext>,
    mission_id: String,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TaskInput {
    id: Option<String>,
    mission_id: String,
    title: String,
    status: McpTaskStatus,
    priority: McpOperationalPriority,
    classification: McpSecurityClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpsertTaskParams {
    auth: Option<McpAuthContext>,
    task: TaskInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MissionList {
    missions: Vec<McpMission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AssetList {
    assets: Vec<McpAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IncidentList {
    incidents: Vec<McpIncident>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TaskList {
    tasks: Vec<McpTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct McpMission {
    id: String,
    tenant_id: String,
    name: String,
    status: McpMissionStatus,
    priority: McpOperationalPriority,
    classification: McpSecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl From<Mission> for McpMission {
    fn from(value: Mission) -> Self {
        Self {
            id: value.id.to_string(),
            tenant_id: value.tenant_id.to_string(),
            name: value.name,
            status: value.status.into(),
            priority: value.priority.into(),
            classification: value.classification.into(),
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct McpTask {
    id: String,
    mission_id: String,
    tenant_id: String,
    title: String,
    status: McpTaskStatus,
    priority: McpOperationalPriority,
    classification: McpSecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl From<Task> for McpTask {
    fn from(value: Task) -> Self {
        Self {
            id: value.id.to_string(),
            mission_id: value.mission_id.to_string(),
            tenant_id: value.tenant_id.to_string(),
            title: value.title,
            status: value.status.into(),
            priority: value.priority.into(),
            classification: value.classification.into(),
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct McpAsset {
    id: String,
    tenant_id: String,
    name: String,
    kind: McpAssetKind,
    status: McpAssetStatus,
    classification: McpSecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl From<Asset> for McpAsset {
    fn from(value: Asset) -> Self {
        Self {
            id: value.id.to_string(),
            tenant_id: value.tenant_id.to_string(),
            name: value.name,
            kind: value.kind.into(),
            status: value.status.into(),
            classification: value.classification.into(),
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct McpIncident {
    id: String,
    tenant_id: String,
    incident_type: McpIncidentType,
    status: McpIncidentStatus,
    summary: String,
    classification: McpSecurityClassification,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl From<Incident> for McpIncident {
    fn from(value: Incident) -> Self {
        Self {
            id: value.id.to_string(),
            tenant_id: value.tenant_id.to_string(),
            incident_type: value.incident_type.into(),
            status: value.status.into(),
            summary: value.summary,
            classification: value.classification.into(),
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }
}

#[tool_router]
impl C2McpService {
    #[tool(
        name = "c2.list_missions",
        description = "List missions for a tenant.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn list_missions(
        &self,
        params: Parameters<ListMissionsParams>,
        meta: Meta,
    ) -> Result<Json<MissionList>, ErrorData> {
        let ListMissionsParams { auth, limit, offset } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        authorize_action(
            &self.policy,
            &auth,
            Permission::ViewMissions,
            SecurityClassification::Unclassified,
            "mission",
            None,
        )?;

        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);
        let missions = MissionRepository::list_by_tenant(&*self.store, auth.subject.tenant_id, limit, offset)
            .await
            .map_err(storage_error)?;
        let missions = missions
            .into_iter()
            .filter(|mission| mission.classification <= auth.subject.clearance)
            .map(McpMission::from)
            .collect();
        Ok(Json(MissionList { missions }))
    }

    #[tool(
        name = "c2.get_mission",
        description = "Fetch a mission by ID.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn get_mission(
        &self,
        params: Parameters<GetByIdParams>,
        meta: Meta,
    ) -> Result<Json<McpMission>, ErrorData> {
        let GetByIdParams { auth, id } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let mission_id = parse_uuid(&id)?;
        let mission_id = MissionId::from_uuid(mission_id);
        let mission = MissionRepository::get(&*self.store, mission_id)
            .await
            .map_err(storage_error)?;
        let Some(mission) = mission else {
            return Err(ErrorData::resource_not_found("mission not found", None));
        };
        authorize_action(
            &self.policy,
            &auth,
            Permission::ViewMissions,
            mission.classification,
            "mission",
            Some(mission.id.to_string()),
        )?;
        Ok(Json(McpMission::from(mission)))
    }

    #[tool(
        name = "c2.upsert_mission",
        description = "Create or update a mission.",
        annotations(read_only_hint = false, idempotent_hint = false, destructive_hint = true)
    )]
    async fn upsert_mission(
        &self,
        params: Parameters<UpsertMissionParams>,
        meta: Meta,
    ) -> Result<Json<McpMission>, ErrorData> {
        let UpsertMissionParams { auth, mission } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let mission_id = match &mission.id {
            Some(value) => MissionId::from_uuid(parse_uuid(value)?),
            None => MissionId::new(),
        };
        let existing = MissionRepository::get(&*self.store, mission_id)
            .await
            .map_err(storage_error)?;
        if let Some(existing) = &existing {
            if existing.tenant_id != auth.subject.tenant_id {
                return Err(ErrorData::invalid_request("tenant mismatch", None));
            }
        }

        let classification: SecurityClassification = mission.classification.into();
        authorize_action(
            &self.policy,
            &auth,
            Permission::EditMissions,
            classification,
            "mission",
            Some(mission_id.to_string()),
        )?;

        let created_at_ms = existing
            .as_ref()
            .map(|mission| mission.created_at_ms)
            .unwrap_or_else(now_epoch_millis);
        let updated_at_ms = now_epoch_millis();
        let mission = Mission {
            id: mission_id,
            tenant_id: auth.subject.tenant_id,
            name: mission.name,
            status: mission.status.into(),
            priority: mission.priority.into(),
            classification,
            created_at_ms,
            updated_at_ms,
        };
        MissionRepository::upsert(&*self.store, mission.clone())
            .await
            .map_err(storage_error)?;
        Ok(Json(McpMission::from(mission)))
    }

    #[tool(
        name = "c2.list_assets",
        description = "List assets for a tenant.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn list_assets(
        &self,
        params: Parameters<ListAssetsParams>,
        meta: Meta,
    ) -> Result<Json<AssetList>, ErrorData> {
        let ListAssetsParams { auth, limit, offset } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        authorize_action(
            &self.policy,
            &auth,
            Permission::DispatchAssets,
            SecurityClassification::Unclassified,
            "asset",
            None,
        )?;
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);
        let assets = AssetRepository::list_by_tenant(&*self.store, auth.subject.tenant_id, limit, offset)
            .await
            .map_err(storage_error)?;
        let assets = assets
            .into_iter()
            .filter(|asset| asset.classification <= auth.subject.clearance)
            .map(McpAsset::from)
            .collect();
        Ok(Json(AssetList { assets }))
    }

    #[tool(
        name = "c2.get_asset",
        description = "Fetch an asset by ID.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn get_asset(
        &self,
        params: Parameters<GetByIdParams>,
        meta: Meta,
    ) -> Result<Json<McpAsset>, ErrorData> {
        let GetByIdParams { auth, id } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let asset_id = AssetId::from_uuid(parse_uuid(&id)?);
        let asset = AssetRepository::get(&*self.store, asset_id)
            .await
            .map_err(storage_error)?;
        let Some(asset) = asset else {
            return Err(ErrorData::resource_not_found("asset not found", None));
        };
        authorize_action(
            &self.policy,
            &auth,
            Permission::DispatchAssets,
            asset.classification,
            "asset",
            Some(asset.id.to_string()),
        )?;
        Ok(Json(McpAsset::from(asset)))
    }

    #[tool(
        name = "c2.upsert_asset",
        description = "Create or update an asset.",
        annotations(read_only_hint = false, idempotent_hint = false, destructive_hint = true)
    )]
    async fn upsert_asset(
        &self,
        params: Parameters<UpsertAssetParams>,
        meta: Meta,
    ) -> Result<Json<McpAsset>, ErrorData> {
        let UpsertAssetParams { auth, asset } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let asset_id = match &asset.id {
            Some(value) => AssetId::from_uuid(parse_uuid(value)?),
            None => AssetId::new(),
        };
        let existing = AssetRepository::get(&*self.store, asset_id)
            .await
            .map_err(storage_error)?;
        if let Some(existing) = &existing {
            if existing.tenant_id != auth.subject.tenant_id {
                return Err(ErrorData::invalid_request("tenant mismatch", None));
            }
        }

        let classification: SecurityClassification = asset.classification.into();
        authorize_action(
            &self.policy,
            &auth,
            Permission::DispatchAssets,
            classification,
            "asset",
            Some(asset_id.to_string()),
        )?;

        let created_at_ms = existing
            .as_ref()
            .map(|asset| asset.created_at_ms)
            .unwrap_or_else(now_epoch_millis);
        let updated_at_ms = now_epoch_millis();
        let asset = Asset {
            id: asset_id,
            tenant_id: auth.subject.tenant_id,
            name: asset.name,
            kind: asset.kind.into(),
            status: asset.status.into(),
            readiness: ReadinessState::default(),
            comms_status: CommsStatus::default(),
            maintenance_state: MaintenanceState::default(),
            unit_id: None,
            capability_ids: Vec::new(),
            classification,
            created_at_ms,
            updated_at_ms,
        };
        AssetRepository::upsert(&*self.store, asset.clone())
            .await
            .map_err(storage_error)?;
        Ok(Json(McpAsset::from(asset)))
    }

    #[tool(
        name = "c2.list_incidents",
        description = "List incidents for a tenant.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn list_incidents(
        &self,
        params: Parameters<ListIncidentsParams>,
        meta: Meta,
    ) -> Result<Json<IncidentList>, ErrorData> {
        let ListIncidentsParams { auth, limit, offset } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        authorize_action(
            &self.policy,
            &auth,
            Permission::ViewIncidents,
            SecurityClassification::Unclassified,
            "incident",
            None,
        )?;
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);
        let incidents =
            IncidentRepository::list_by_tenant(&*self.store, auth.subject.tenant_id, limit, offset)
                .await
                .map_err(storage_error)?;
        let incidents = incidents
            .into_iter()
            .filter(|incident| incident.classification <= auth.subject.clearance)
            .map(McpIncident::from)
            .collect();
        Ok(Json(IncidentList { incidents }))
    }

    #[tool(
        name = "c2.get_incident",
        description = "Fetch an incident by ID.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn get_incident(
        &self,
        params: Parameters<GetByIdParams>,
        meta: Meta,
    ) -> Result<Json<McpIncident>, ErrorData> {
        let GetByIdParams { auth, id } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let incident_id = IncidentId::from_uuid(parse_uuid(&id)?);
        let incident = IncidentRepository::get(&*self.store, incident_id)
            .await
            .map_err(storage_error)?;
        let Some(incident) = incident else {
            return Err(ErrorData::resource_not_found("incident not found", None));
        };
        authorize_action(
            &self.policy,
            &auth,
            Permission::ViewIncidents,
            incident.classification,
            "incident",
            Some(incident.id.to_string()),
        )?;
        Ok(Json(McpIncident::from(incident)))
    }

    #[tool(
        name = "c2.upsert_incident",
        description = "Create or update an incident.",
        annotations(read_only_hint = false, idempotent_hint = false, destructive_hint = true)
    )]
    async fn upsert_incident(
        &self,
        params: Parameters<UpsertIncidentParams>,
        meta: Meta,
    ) -> Result<Json<McpIncident>, ErrorData> {
        let UpsertIncidentParams { auth, incident } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let incident_id = match &incident.id {
            Some(value) => IncidentId::from_uuid(parse_uuid(value)?),
            None => IncidentId::new(),
        };
        let existing = IncidentRepository::get(&*self.store, incident_id)
            .await
            .map_err(storage_error)?;
        if let Some(existing) = &existing {
            if existing.tenant_id != auth.subject.tenant_id {
                return Err(ErrorData::invalid_request("tenant mismatch", None));
            }
        }

        let classification: SecurityClassification = incident.classification.into();
        authorize_action(
            &self.policy,
            &auth,
            Permission::IngestData,
            classification,
            "incident",
            Some(incident_id.to_string()),
        )?;

        let created_at_ms = existing
            .as_ref()
            .map(|incident| incident.created_at_ms)
            .unwrap_or_else(now_epoch_millis);
        let updated_at_ms = now_epoch_millis();
        let incident = Incident {
            id: incident_id,
            tenant_id: auth.subject.tenant_id,
            incident_type: incident.incident_type.into(),
            status: incident.status.into(),
            summary: incident.summary,
            classification,
            created_at_ms,
            updated_at_ms,
        };
        IncidentRepository::upsert(&*self.store, incident.clone())
            .await
            .map_err(storage_error)?;
        Ok(Json(McpIncident::from(incident)))
    }

    #[tool(
        name = "c2.list_tasks",
        description = "List tasks for a mission.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn list_tasks(
        &self,
        params: Parameters<ListTasksParams>,
        meta: Meta,
    ) -> Result<Json<TaskList>, ErrorData> {
        let ListTasksParams {
            auth,
            mission_id,
            limit,
            offset,
        } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        authorize_action(
            &self.policy,
            &auth,
            Permission::ViewMissions,
            SecurityClassification::Unclassified,
            "task",
            None,
        )?;
        let mission_id = MissionId::from_uuid(parse_uuid(&mission_id)?);
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);
        let tasks = TaskRepository::list_by_mission(&*self.store, mission_id, limit, offset)
            .await
            .map_err(storage_error)?;
        let tasks = tasks
            .into_iter()
            .filter(|task| task.classification <= auth.subject.clearance)
            .map(McpTask::from)
            .collect();
        Ok(Json(TaskList { tasks }))
    }

    #[tool(
        name = "c2.get_task",
        description = "Fetch a task by ID.",
        annotations(read_only_hint = true, idempotent_hint = true, destructive_hint = false)
    )]
    async fn get_task(
        &self,
        params: Parameters<GetByIdParams>,
        meta: Meta,
    ) -> Result<Json<McpTask>, ErrorData> {
        let GetByIdParams { auth, id } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let task_id = TaskId::from_uuid(parse_uuid(&id)?);
        let task = TaskRepository::get(&*self.store, task_id)
            .await
            .map_err(storage_error)?;
        let Some(task) = task else {
            return Err(ErrorData::resource_not_found("task not found", None));
        };
        authorize_action(
            &self.policy,
            &auth,
            Permission::ViewMissions,
            task.classification,
            "task",
            Some(task.id.to_string()),
        )?;
        Ok(Json(McpTask::from(task)))
    }

    #[tool(
        name = "c2.upsert_task",
        description = "Create or update a task.",
        annotations(read_only_hint = false, idempotent_hint = false, destructive_hint = true)
    )]
    async fn upsert_task(
        &self,
        params: Parameters<UpsertTaskParams>,
        meta: Meta,
    ) -> Result<Json<McpTask>, ErrorData> {
        let UpsertTaskParams { auth, task } = params.0;
        let auth = resolve_auth(auth, &meta, self.default_auth.as_ref())?;
        let task_id = match &task.id {
            Some(value) => TaskId::from_uuid(parse_uuid(value)?),
            None => TaskId::new(),
        };
        let mission_id = MissionId::from_uuid(parse_uuid(&task.mission_id)?);
        let mission = MissionRepository::get(&*self.store, mission_id)
            .await
            .map_err(storage_error)?;
        let Some(mission) = mission else {
            return Err(ErrorData::resource_not_found("mission not found", None));
        };
        if mission.tenant_id != auth.subject.tenant_id {
            return Err(ErrorData::invalid_request("tenant mismatch", None));
        }
        let existing = TaskRepository::get(&*self.store, task_id)
            .await
            .map_err(storage_error)?;
        if let Some(existing) = &existing {
            if existing.tenant_id != auth.subject.tenant_id {
                return Err(ErrorData::invalid_request("tenant mismatch", None));
            }
        }

        let classification: SecurityClassification = task.classification.into();
        authorize_action(
            &self.policy,
            &auth,
            Permission::EditMissions,
            classification,
            "task",
            Some(task_id.to_string()),
        )?;

        let created_at_ms = existing
            .as_ref()
            .map(|task| task.created_at_ms)
            .unwrap_or_else(now_epoch_millis);
        let updated_at_ms = now_epoch_millis();
        let task = Task {
            id: task_id,
            mission_id,
            tenant_id: auth.subject.tenant_id,
            title: task.title,
            status: task.status.into(),
            priority: task.priority.into(),
            classification,
            created_at_ms,
            updated_at_ms,
        };
        TaskRepository::upsert(&*self.store, task.clone())
            .await
            .map_err(storage_error)?;
        Ok(Json(McpTask::from(task)))
    }
}

#[tool_handler]
impl ServerHandler for C2McpService {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .build();
        ServerInfo {
            protocol_version: Default::default(),
            capabilities,
            server_info: rmcp::model::Implementation {
                name: "c2-mcp".to_string(),
                title: Some("C2 MCP Gateway".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Provide auth context in tool parameters (auth) or in _meta.auth. If omitted, the server uses C2_MCP_* default auth when configured."
                    .to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParam>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let auth = resolve_auth(None, &context.meta, self.default_auth.as_ref())?;
        authorize_action(
            &self.policy,
            &auth,
            Permission::ViewMissions,
            SecurityClassification::Unclassified,
            "resource.mission",
            None,
        )?;

        let offset = request
            .and_then(|value| value.cursor)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let limit = 50;
        let missions =
            MissionRepository::list_by_tenant(&*self.store, auth.subject.tenant_id, limit, offset)
                .await
                .map_err(storage_error)?;
        let resources = missions
            .into_iter()
            .filter(|mission| mission.classification <= auth.subject.clearance)
            .map(mission_resource)
            .collect::<Vec<_>>();

        let next_cursor = if resources.len() == limit {
            Some((offset + limit).to_string())
        } else {
            None
        };
        Ok(ListResourcesResult {
            resources,
            meta: None,
            next_cursor,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![
                resource_template("c2://mission/{id}", "mission", "Mission record"),
                resource_template("c2://task/{id}", "task", "Task record"),
                resource_template("c2://asset/{id}", "asset", "Asset record"),
                resource_template("c2://incident/{id}", "incident", "Incident record"),
            ],
            meta: None,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let auth = resolve_auth(None, &context.meta, self.default_auth.as_ref())?;
        let (kind, id) = parse_resource_uri(&request.uri)?;
        match kind {
            ResourceKind::Mission => {
                let mission = MissionRepository::get(&*self.store, MissionId::from_uuid(id))
                    .await
                    .map_err(storage_error)?;
                let Some(mission) = mission else {
                    return Err(ErrorData::resource_not_found("mission not found", None));
                };
                authorize_action(
                    &self.policy,
                    &auth,
                    Permission::ViewMissions,
                    mission.classification,
                    "mission",
                    Some(mission.id.to_string()),
                )?;
                let payload = serde_json::to_string(&McpMission::from(mission))
                    .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
                Ok(ReadResourceResult {
                    contents: vec![resource_contents(&request.uri, payload)],
                })
            }
            ResourceKind::Task => {
                let task = TaskRepository::get(&*self.store, TaskId::from_uuid(id))
                    .await
                    .map_err(storage_error)?;
                let Some(task) = task else {
                    return Err(ErrorData::resource_not_found("task not found", None));
                };
                authorize_action(
                    &self.policy,
                    &auth,
                    Permission::ViewMissions,
                    task.classification,
                    "task",
                    Some(task.id.to_string()),
                )?;
                let payload = serde_json::to_string(&McpTask::from(task))
                    .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
                Ok(ReadResourceResult {
                    contents: vec![resource_contents(&request.uri, payload)],
                })
            }
            ResourceKind::Asset => {
                let asset = AssetRepository::get(&*self.store, AssetId::from_uuid(id))
                    .await
                    .map_err(storage_error)?;
                let Some(asset) = asset else {
                    return Err(ErrorData::resource_not_found("asset not found", None));
                };
                authorize_action(
                    &self.policy,
                    &auth,
                    Permission::DispatchAssets,
                    asset.classification,
                    "asset",
                    Some(asset.id.to_string()),
                )?;
                let payload = serde_json::to_string(&McpAsset::from(asset))
                    .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
                Ok(ReadResourceResult {
                    contents: vec![resource_contents(&request.uri, payload)],
                })
            }
            ResourceKind::Incident => {
                let incident = IncidentRepository::get(&*self.store, IncidentId::from_uuid(id))
                    .await
                    .map_err(storage_error)?;
                let Some(incident) = incident else {
                    return Err(ErrorData::resource_not_found("incident not found", None));
                };
                authorize_action(
                    &self.policy,
                    &auth,
                    Permission::ViewIncidents,
                    incident.classification,
                    "incident",
                    Some(incident.id.to_string()),
                )?;
                let payload = serde_json::to_string(&McpIncident::from(incident))
                    .map_err(|err| ErrorData::internal_error(err.to_string(), None))?;
                Ok(ReadResourceResult {
                    contents: vec![resource_contents(&request.uri, payload)],
                })
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServiceConfig::from_env("c2-mcp");
    let obs_config = ObservabilityConfig {
        service_name: config.service_name.clone(),
        environment: config.environment.to_string(),
        log_level: config.log_level.clone(),
        metrics_addr: config.metrics_addr.clone(),
    };
    let handle = init(&obs_config);
    log_startup(&handle, &obs_config.environment);

    let surreal_config = SurrealConfig::from_env();
    let store = SurrealStore::connect_with_retry(&surreal_config).await?;
    let policy = BasicPolicyEngine::with_default_rules();
    let service = C2McpService::new(store, policy);

    let session_manager = Arc::new(LocalSessionManager::default());
    let http_service = StreamableHttpService::new(
        {
            let service = service.clone();
            move || Ok(service.clone())
        },
        session_manager,
        StreamableHttpServerConfig::default(),
    );
    let app = Router::new().route("/mcp", any_service(http_service));
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("c2-mcp http listening on {}", config.bind_addr);

    let shutdown = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to install ctrl-c handler: {}", err);
        }
    };
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    Ok(())
}

fn parse_uuid(value: &str) -> Result<Uuid, ErrorData> {
    Uuid::parse_str(value).map_err(|_| ErrorData::invalid_params("invalid UUID", None))
}

fn parse_auth(auth: &McpAuthContext) -> Result<AuthorizedContext, ErrorData> {
    let tenant_id = parse_uuid(&auth.tenant_id)?;
    let user_id = parse_uuid(&auth.user_id)?;
    let roles = auth
        .roles
        .iter()
        .map(|value| Role::from_str(value).map_err(|_| ErrorData::invalid_params("invalid role", None)))
        .collect::<Result<Vec<_>, _>>()?;
    let permissions = auth
        .permissions
        .iter()
        .map(|value| {
            Permission::from_str(value)
                .map_err(|_| ErrorData::invalid_params("invalid permission", None))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if roles.is_empty() || permissions.is_empty() {
        return Err(ErrorData::invalid_request("missing roles or permissions", None));
    }

    let clearance = auth
        .clearance
        .clone()
        .map(SecurityClassification::from)
        .unwrap_or(SecurityClassification::Unclassified);

    Ok(AuthorizedContext {
        subject: Subject {
            tenant_id: TenantId::from_uuid(tenant_id),
            user_id: c2_core::UserId::from_uuid(user_id),
            roles,
            clearance,
        },
        permissions,
    })
}

fn authorize_action(
    policy: &BasicPolicyEngine,
    auth: &AuthorizedContext,
    permission: Permission,
    classification: SecurityClassification,
    resource_type: &str,
    resource_id: Option<String>,
) -> Result<(), ErrorData> {
    if !auth.permissions.contains(&permission) {
        return Err(ErrorData::invalid_request("permission denied", None));
    }
    let request = PolicyRequest {
        subject: auth.subject.clone(),
        action: permission,
        resource: ResourceDescriptor {
            resource_type: resource_type.to_string(),
            resource_id,
        },
        classification,
        context: PolicyContext {
            tenant_id: auth.subject.tenant_id,
            mission_id: None,
            incident_id: None,
            tags: vec![],
        },
    };
    match policy.evaluate(&request) {
        PolicyDecision::Permit => Ok(()),
        PolicyDecision::Deny => Err(ErrorData::invalid_request("policy denied", None)),
        PolicyDecision::Indeterminate => {
            Err(ErrorData::internal_error("policy indeterminate", None))
        }
    }
}

fn storage_error(err: StorageError) -> ErrorData {
    ErrorData::internal_error(err.message, None)
}

fn resource_template(uri_template: &str, name: &str, description: &str) -> ResourceTemplate {
    let raw = RawResourceTemplate {
        uri_template: uri_template.to_string(),
        name: name.to_string(),
        title: Some(name.to_string()),
        description: Some(description.to_string()),
        mime_type: Some("application/json".to_string()),
    };
    Annotated::new(raw, None)
}

fn mission_resource(mission: Mission) -> Resource {
    let mut raw = RawResource::new(format!("c2://mission/{}", mission.id), mission.name.clone());
    raw.title = Some(mission.name);
    raw.description = Some("Mission record".to_string());
    raw.mime_type = Some("application/json".to_string());
    Annotated::new(raw, None)
}

fn resource_contents(uri: &str, payload: String) -> ResourceContents {
    ResourceContents::TextResourceContents {
        uri: uri.to_string(),
        mime_type: Some("application/json".to_string()),
        text: payload,
        meta: None,
    }
}

fn resolve_auth(
    params_auth: Option<McpAuthContext>,
    meta: &Meta,
    default_auth: Option<&AuthorizedContext>,
) -> Result<AuthorizedContext, ErrorData> {
    if let Some(auth) = params_auth {
        return parse_auth(&auth);
    }
    if let Some(auth) = auth_from_meta(meta)? {
        return parse_auth(&auth);
    }
    if let Some(auth) = default_auth {
        return Ok(auth.clone());
    }
    Err(ErrorData::invalid_params("missing auth context", None))
}

fn auth_from_meta(meta: &Meta) -> Result<Option<McpAuthContext>, ErrorData> {
    let auth_value = match meta.get("auth") {
        Some(value) => value.clone(),
        None => return Ok(None),
    };
    let auth = serde_json::from_value(auth_value)
        .map_err(|_| ErrorData::invalid_params("invalid auth in meta", None))?;
    Ok(Some(auth))
}

fn load_default_auth() -> Option<AuthorizedContext> {
    let tenant_id = env::var("C2_MCP_TENANT_ID").ok()?;
    let user_id = env::var("C2_MCP_USER_ID").ok()?;
    let roles = env::var("C2_MCP_ROLES").ok()?;
    let permissions = env::var("C2_MCP_PERMISSIONS").ok()?;
    let clearance = match env::var("C2_MCP_CLEARANCE").ok() {
        Some(value) => match parse_clearance(&value) {
            Some(clearance) => Some(clearance),
            None => {
                tracing::warn!("invalid C2_MCP_CLEARANCE value: {}", value);
                None
            }
        },
        None => None,
    };
    let auth = McpAuthContext {
        tenant_id,
        user_id,
        roles: split_csv(&roles),
        permissions: split_csv(&permissions),
        clearance,
    };
    match parse_auth(&auth) {
        Ok(auth) => {
            tracing::info!("c2-mcp default auth loaded from C2_MCP_* env");
            Some(auth)
        }
        Err(err) => {
            tracing::error!("invalid C2_MCP_* auth configuration: {}", err.message);
            None
        }
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn parse_clearance(value: &str) -> Option<McpSecurityClassification> {
    match value.trim().to_ascii_lowercase().as_str() {
        "unclassified" => Some(McpSecurityClassification::Unclassified),
        "controlled" => Some(McpSecurityClassification::Controlled),
        "restricted" => Some(McpSecurityClassification::Restricted),
        "confidential" => Some(McpSecurityClassification::Confidential),
        "secret" => Some(McpSecurityClassification::Secret),
        "top_secret" | "top-secret" => Some(McpSecurityClassification::TopSecret),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum ResourceKind {
    Mission,
    Task,
    Asset,
    Incident,
}

fn parse_resource_uri(uri: &str) -> Result<(ResourceKind, Uuid), ErrorData> {
    let uri = uri.strip_prefix("c2://").ok_or_else(|| {
        ErrorData::resource_not_found("unsupported resource uri", None)
    })?;
    let mut parts = uri.split('/');
    let kind = parts
        .next()
        .ok_or_else(|| ErrorData::resource_not_found("missing resource type", None))?;
    let id = parts
        .next()
        .ok_or_else(|| ErrorData::resource_not_found("missing resource id", None))?;
    let parsed = parse_uuid(id)?;
    let kind = match kind {
        "mission" => ResourceKind::Mission,
        "task" => ResourceKind::Task,
        "asset" => ResourceKind::Asset,
        "incident" => ResourceKind::Incident,
        _ => return Err(ErrorData::resource_not_found("unknown resource type", None)),
    };
    Ok((kind, parsed))
}
