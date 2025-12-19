use actix_web::{HttpRequest, HttpResponse};
use c2_core::{SecurityClassification, TenantId, UserId};
use c2_identity::{Permission, Role, Subject};
use c2_policy::{
    BasicPolicyEngine, PolicyContext, PolicyDecision, PolicyEngine, PolicyRequest,
    ResourceDescriptor,
};
use std::str::FromStr;
use uuid::Uuid;

use crate::routes::common::{bad_request, forbidden, unauthorized};

pub struct AuthInfo {
    pub subject: Subject,
}

pub fn authorize_request(
    req: &HttpRequest,
    engine: &BasicPolicyEngine,
    permission: Permission,
    classification: SecurityClassification,
) -> Result<AuthInfo, HttpResponse> {
    let tenant_id = parse_uuid_header(req, "x-c2-tenant-id")?;
    let user_id = parse_uuid_header(req, "x-c2-user-id")?;
    let roles = parse_list_header(req, "x-c2-roles")?
        .into_iter()
        .map(|value| Role::from_str(&value).map_err(|_| bad_request("invalid role")))
        .collect::<Result<Vec<_>, _>>()?;
    let permissions = parse_list_header(req, "x-c2-permissions")?
        .into_iter()
        .map(|value| {
            Permission::from_str(&value).map_err(|_| bad_request("invalid permission"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let clearance = header_value(req, "x-c2-clearance")
        .and_then(|value| SecurityClassification::from_str(&value).ok())
        .unwrap_or(SecurityClassification::Unclassified);

    if roles.is_empty() || permissions.is_empty() {
        return Err(unauthorized("missing roles or permissions"));
    }

    if !permissions.contains(&permission) {
        return Err(forbidden("permission denied"));
    }

    let subject = Subject {
        tenant_id: TenantId::from_uuid(tenant_id),
        user_id: UserId::from_uuid(user_id),
        roles,
        clearance,
    };

    let request = PolicyRequest {
        subject: subject.clone(),
        action: permission,
        resource: ResourceDescriptor {
            resource_type: req.path().to_string(),
            resource_id: None,
        },
        classification,
        context: PolicyContext {
            tenant_id: subject.tenant_id,
            mission_id: None,
            incident_id: None,
            tags: vec![],
        },
    };

    match engine.evaluate(&request) {
        PolicyDecision::Permit => Ok(AuthInfo { subject }),
        PolicyDecision::Deny => Err(forbidden("policy denied")),
        PolicyDecision::Indeterminate => Err(unauthorized("policy indeterminate")),
    }
}

fn parse_uuid_header(req: &HttpRequest, name: &str) -> Result<Uuid, HttpResponse> {
    let value = header_value(req, name).ok_or_else(|| unauthorized("missing auth header"))?;
    Uuid::parse_str(&value).map_err(|_| bad_request("invalid UUID"))
}

fn parse_list_header(req: &HttpRequest, name: &str) -> Result<Vec<String>, HttpResponse> {
    let value = header_value(req, name).ok_or_else(|| unauthorized("missing auth header"))?;
    Ok(value
        .split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect())
}

fn header_value(req: &HttpRequest, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}
