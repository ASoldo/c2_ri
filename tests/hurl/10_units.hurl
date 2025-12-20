POST {{base_url}}/v1/capabilities
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{capability_id}}",
  "tenant_id": "{{tenant_id}}",
  "code": "transport",
  "name": "Transport",
  "category": "mobility",
  "description": "Secure transport and evacuation.",
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
  "id": "{{team_id}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Foxtrot Wing",
  "callsign": "FOXTROT",
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
  "id": "{{unit_id}}",
  "tenant_id": "{{tenant_id}}",
  "classification": "restricted",
  "callsign": "F-12",
  "display_name": "Foxtrot Unit 12",
  "readiness": "limited",
  "comms_status": "intermittent",
  "team_id": "{{team_id}}",
  "capability_ids": ["{{capability_id}}"],
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

GET {{base_url}}/v1/units/{{unit_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json
jsonpath "$.id" == "{{unit_id}}"

GET {{base_url}}/v1/units?tenant_id={{tenant_id}}&limit=10&offset=0
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json

DELETE {{base_url}}/v1/units/{{unit_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204

DELETE {{base_url}}/v1/teams/{{team_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204

DELETE {{base_url}}/v1/capabilities/{{capability_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204
