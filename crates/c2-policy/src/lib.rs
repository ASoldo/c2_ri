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

#[derive(Debug, Clone)]
pub struct BasicPolicyEngine {
    rules: Vec<PolicyRule>,
}

impl BasicPolicyEngine {
    pub fn new(rules: Vec<PolicyRule>) -> Self {
        Self { rules }
    }

    pub fn with_default_rules() -> Self {
        Self::new(default_rules())
    }

    fn matches_rule(&self, request: &PolicyRequest, rule: &PolicyRule) -> bool {
        if request.subject.clearance < rule.minimum_clearance {
            return false;
        }
        if request.subject.clearance < request.classification {
            return false;
        }
        if !rule.required_permissions.is_empty()
            && !rule.required_permissions.contains(&request.action)
        {
            return false;
        }
        if !rule.required_roles.is_empty()
            && !rule
                .required_roles
                .iter()
                .any(|role| request.subject.has_role(*role))
        {
            return false;
        }
        true
    }
}

impl PolicyEngine for BasicPolicyEngine {
    fn evaluate(&self, request: &PolicyRequest) -> PolicyDecision {
        if self.rules.is_empty() {
            return if request.subject.clearance >= request.classification
            {
                PolicyDecision::Permit
            } else {
                PolicyDecision::Deny
            };
        }

        if self
            .rules
            .iter()
            .any(|rule| self.matches_rule(request, rule))
        {
            PolicyDecision::Permit
        } else {
            PolicyDecision::Deny
        }
    }
}

fn default_rules() -> Vec<PolicyRule> {
    vec![
        PolicyRule {
            id: "view_missions".to_string(),
            description: "View mission data".to_string(),
            required_roles: vec![
                Role::SystemAdmin,
                Role::MissionCommander,
                Role::Operations,
                Role::Analyst,
                Role::FieldResponder,
                Role::Observer,
            ],
            required_permissions: vec![Permission::ViewMissions],
            minimum_clearance: SecurityClassification::Unclassified,
        },
        PolicyRule {
            id: "edit_missions".to_string(),
            description: "Create or update missions".to_string(),
            required_roles: vec![Role::SystemAdmin, Role::MissionCommander, Role::Operations],
            required_permissions: vec![Permission::EditMissions],
            minimum_clearance: SecurityClassification::Restricted,
        },
        PolicyRule {
            id: "dispatch_assets".to_string(),
            description: "Dispatch and reassign assets".to_string(),
            required_roles: vec![Role::SystemAdmin, Role::MissionCommander, Role::Operations],
            required_permissions: vec![Permission::DispatchAssets],
            minimum_clearance: SecurityClassification::Restricted,
        },
        PolicyRule {
            id: "view_units".to_string(),
            description: "View unit registry".to_string(),
            required_roles: vec![
                Role::SystemAdmin,
                Role::MissionCommander,
                Role::Operations,
                Role::Analyst,
                Role::FieldResponder,
                Role::Observer,
            ],
            required_permissions: vec![Permission::ViewUnits],
            minimum_clearance: SecurityClassification::Unclassified,
        },
        PolicyRule {
            id: "edit_units".to_string(),
            description: "Create or update units".to_string(),
            required_roles: vec![Role::SystemAdmin, Role::MissionCommander, Role::Operations],
            required_permissions: vec![Permission::EditUnits],
            minimum_clearance: SecurityClassification::Restricted,
        },
        PolicyRule {
            id: "view_teams".to_string(),
            description: "View team registry".to_string(),
            required_roles: vec![
                Role::SystemAdmin,
                Role::MissionCommander,
                Role::Operations,
                Role::Analyst,
                Role::FieldResponder,
                Role::Observer,
            ],
            required_permissions: vec![Permission::ViewTeams],
            minimum_clearance: SecurityClassification::Unclassified,
        },
        PolicyRule {
            id: "edit_teams".to_string(),
            description: "Create or update teams".to_string(),
            required_roles: vec![Role::SystemAdmin, Role::MissionCommander, Role::Operations],
            required_permissions: vec![Permission::EditTeams],
            minimum_clearance: SecurityClassification::Restricted,
        },
        PolicyRule {
            id: "view_capabilities".to_string(),
            description: "View capability catalog".to_string(),
            required_roles: vec![
                Role::SystemAdmin,
                Role::MissionCommander,
                Role::Operations,
                Role::Analyst,
                Role::FieldResponder,
                Role::Observer,
            ],
            required_permissions: vec![Permission::ViewCapabilities],
            minimum_clearance: SecurityClassification::Unclassified,
        },
        PolicyRule {
            id: "edit_capabilities".to_string(),
            description: "Create or update capabilities".to_string(),
            required_roles: vec![Role::SystemAdmin, Role::MissionCommander, Role::Operations],
            required_permissions: vec![Permission::EditCapabilities],
            minimum_clearance: SecurityClassification::Restricted,
        },
        PolicyRule {
            id: "view_incidents".to_string(),
            description: "View incident feeds".to_string(),
            required_roles: vec![
                Role::SystemAdmin,
                Role::MissionCommander,
                Role::Operations,
                Role::FieldResponder,
                Role::Analyst,
            ],
            required_permissions: vec![Permission::ViewIncidents],
            minimum_clearance: SecurityClassification::Unclassified,
        },
        PolicyRule {
            id: "ingest_data".to_string(),
            description: "Ingest incident and sensor data".to_string(),
            required_roles: vec![Role::SystemAdmin, Role::Operations, Role::Analyst],
            required_permissions: vec![Permission::IngestData],
            minimum_clearance: SecurityClassification::Restricted,
        },
        PolicyRule {
            id: "access_classified".to_string(),
            description: "Access classified data".to_string(),
            required_roles: vec![Role::SystemAdmin, Role::MissionCommander, Role::Operations],
            required_permissions: vec![Permission::AccessClassified],
            minimum_clearance: SecurityClassification::Secret,
        },
        PolicyRule {
            id: "admin".to_string(),
            description: "Administrative actions".to_string(),
            required_roles: vec![Role::SystemAdmin],
            required_permissions: vec![Permission::Admin],
            minimum_clearance: SecurityClassification::Restricted,
        },
    ]
}
