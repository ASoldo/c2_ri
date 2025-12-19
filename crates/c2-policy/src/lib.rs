use c2_core::{IncidentId, MissionId, SecurityClassification, TenantId};
use c2_identity::{Permission, Role, Subject};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Permit,
    Deny,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDescriptor {
    pub resource_type: String,
    pub resource_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    pub tenant_id: TenantId,
    pub mission_id: Option<MissionId>,
    pub incident_id: Option<IncidentId>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub subject: Subject,
    pub action: Permission,
    pub resource: ResourceDescriptor,
    pub classification: SecurityClassification,
    pub context: PolicyContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub description: String,
    pub required_roles: Vec<Role>,
    pub required_permissions: Vec<Permission>,
    pub minimum_clearance: SecurityClassification,
}

pub trait PolicyEngine {
    fn evaluate(&self, request: &PolicyRequest) -> PolicyDecision;
}
