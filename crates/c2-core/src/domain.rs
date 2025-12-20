use crate::classification::SecurityClassification;
use crate::ids::{
    AssetId, CapabilityId, IncidentId, MissionId, TaskId, TeamId, TenantId, UnitId,
};
use crate::time::EpochMillis;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalPriority {
    Routine,
    Elevated,
    Urgent,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    Planned,
    Active,
    Suspended,
    Completed,
    Aborted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Available,
    Assigned,
    Degraded,
    Maintenance,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessState {
    Ready,
    Limited,
    Degraded,
    Unavailable,
}

impl Default for ReadinessState {
    fn default() -> Self {
        Self::Ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommsStatus {
    Online,
    Intermittent,
    Offline,
    Unknown,
}

impl Default for CommsStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceState {
    None,
    Scheduled,
    InProgress,
    Deferred,
    Complete,
}

impl Default for MaintenanceState {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentType {
    Defense,
    Fire,
    Medical,
    Hazmat,
    Rescue,
    PublicSafety,
    Infrastructure,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Reported,
    Verified,
    Responding,
    Contained,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: MissionId,
    pub tenant_id: TenantId,
    pub name: String,
    pub status: MissionStatus,
    pub priority: OperationalPriority,
    pub classification: SecurityClassification,
    pub created_at_ms: EpochMillis,
    pub updated_at_ms: EpochMillis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub mission_id: MissionId,
    pub tenant_id: TenantId,
    pub title: String,
    pub status: TaskStatus,
    pub priority: OperationalPriority,
    pub classification: SecurityClassification,
    pub created_at_ms: EpochMillis,
    pub updated_at_ms: EpochMillis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub tenant_id: TenantId,
    pub name: String,
    pub kind: AssetKind,
    pub status: AssetStatus,
    #[serde(default)]
    pub readiness: ReadinessState,
    #[serde(default)]
    pub comms_status: CommsStatus,
    #[serde(default)]
    pub maintenance_state: MaintenanceState,
    #[serde(default)]
    pub unit_id: Option<UnitId>,
    #[serde(default)]
    pub capability_ids: Vec<CapabilityId>,
    pub classification: SecurityClassification,
    pub created_at_ms: EpochMillis,
    pub updated_at_ms: EpochMillis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub tenant_id: TenantId,
    pub classification: SecurityClassification,
    pub callsign: Option<String>,
    pub display_name: String,
    #[serde(default)]
    pub readiness: ReadinessState,
    #[serde(default)]
    pub comms_status: CommsStatus,
    #[serde(default)]
    pub team_id: Option<TeamId>,
    #[serde(default)]
    pub capability_ids: Vec<CapabilityId>,
    pub created_at_ms: EpochMillis,
    pub updated_at_ms: EpochMillis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub tenant_id: TenantId,
    pub name: String,
    pub callsign: Option<String>,
    pub classification: SecurityClassification,
    pub created_at_ms: EpochMillis,
    pub updated_at_ms: EpochMillis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: CapabilityId,
    pub tenant_id: TenantId,
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub classification: SecurityClassification,
    pub created_at_ms: EpochMillis,
    pub updated_at_ms: EpochMillis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: IncidentId,
    pub tenant_id: TenantId,
    pub incident_type: IncidentType,
    pub status: IncidentStatus,
    pub summary: String,
    pub classification: SecurityClassification,
    pub created_at_ms: EpochMillis,
    pub updated_at_ms: EpochMillis,
}
