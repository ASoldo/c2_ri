use c2_core::{SecurityClassification, TenantId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    SystemAdmin,
    MissionCommander,
    Operations,
    Analyst,
    FieldResponder,
    Integrator,
    Observer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ViewMissions,
    EditMissions,
    DispatchAssets,
    ViewIncidents,
    ManageUsers,
    ManagePolicies,
    AccessClassified,
    IngestData,
    ExportData,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub roles: Vec<Role>,
    pub clearance: SecurityClassification,
}

impl Subject {
    pub fn has_role(&self, role: Role) -> bool {
        self.roles.iter().any(|candidate| *candidate == role)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub subject: Subject,
    pub permissions: Vec<Permission>,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

impl AuthContext {
    pub fn allows(&self, permission: Permission) -> bool {
        self.permissions.iter().any(|candidate| *candidate == permission)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub issuer: String,
    pub audience: String,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub roles: Vec<Role>,
    pub clearance: SecurityClassification,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}
