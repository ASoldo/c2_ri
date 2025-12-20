POST {{base_url}}/v1/missions
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{mission_id_alpha}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Operation Sentinel",
  "status": "active",
  "priority": "critical",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/missions
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{mission_id_bravo}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Operation Harborline",
  "status": "planned",
  "priority": "elevated",
  "classification": "unclassified",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/capabilities
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{capability_id_alpha}}",
  "tenant_id": "{{tenant_id}}",
  "code": "medevac",
  "name": "Medical Evacuation",
  "category": "medical",
  "description": "Advanced evacuation and triage support.",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/teams
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{team_id_alpha}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Alpha Response Team",
  "callsign": "ALPHA",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/units
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{unit_id_alpha}}",
  "tenant_id": "{{tenant_id}}",
  "classification": "restricted",
  "callsign": "R-31",
  "display_name": "Rescue Unit 31",
  "readiness": "ready",
  "comms_status": "online",
  "team_id": "{{team_id_alpha}}",
  "capability_ids": ["{{capability_id_alpha}}"],
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/assets
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{asset_id_alpha}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Rescue-31",
  "kind": "vehicle",
  "status": "available",
  "readiness": "ready",
  "comms_status": "online",
  "maintenance_state": "none",
  "unit_id": "{{unit_id_alpha}}",
  "capability_ids": ["{{capability_id_alpha}}"],
  "classification": "unclassified",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/assets
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{asset_id_bravo}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Drone-07",
  "kind": "drone",
  "status": "degraded",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/assets
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{asset_id_charlie}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Relay-Zulu",
  "kind": "comms_relay",
  "status": "maintenance",
  "classification": "confidential",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/incidents
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{incident_id_alpha}}",
  "tenant_id": "{{tenant_id}}",
  "incident_type": "rescue",
  "status": "responding",
  "summary": "Alpine extraction requested for stranded survey team.",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/incidents
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{incident_id_bravo}}",
  "tenant_id": "{{tenant_id}}",
  "incident_type": "hazmat",
  "status": "reported",
  "summary": "Chemical spill reported at north interchange. Teams staging.",
  "classification": "confidential",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200
