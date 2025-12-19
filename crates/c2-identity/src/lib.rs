use c2_core::{SecurityClassification, TenantId, UserId};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

impl FromStr for Role {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "system_admin" | "systemadmin" => Ok(Self::SystemAdmin),
            "mission_commander" | "missioncommander" => Ok(Self::MissionCommander),
            "operations" | "ops" => Ok(Self::Operations),
            "analyst" => Ok(Self::Analyst),
            "field_responder" | "fieldresponder" => Ok(Self::FieldResponder),
            "integrator" => Ok(Self::Integrator),
            "observer" => Ok(Self::Observer),
            _ => Err(()),
        }
    }
}

impl FromStr for Permission {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "view_missions" | "viewmissions" => Ok(Self::ViewMissions),
            "edit_missions" | "editmissions" => Ok(Self::EditMissions),
            "dispatch_assets" | "dispatchassets" => Ok(Self::DispatchAssets),
            "view_incidents" | "viewincidents" => Ok(Self::ViewIncidents),
            "manage_users" | "manageusers" => Ok(Self::ManageUsers),
            "manage_policies" | "managepolicies" => Ok(Self::ManagePolicies),
            "access_classified" | "accessclassified" => Ok(Self::AccessClassified),
            "ingest_data" | "ingestdata" => Ok(Self::IngestData),
            "export_data" | "exportdata" => Ok(Self::ExportData),
            "admin" => Ok(Self::Admin),
            _ => Err(()),
        }
    }
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
