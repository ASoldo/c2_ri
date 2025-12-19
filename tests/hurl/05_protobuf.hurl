POST {{base_url}}/v1/missions
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{mission_id}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Operation Signal",
  "status": "active",
  "priority": "elevated",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

POST {{base_url}}/v1/tasks
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{task_id}}",
  "mission_id": "{{mission_id}}",
  "tenant_id": "{{tenant_id}}",
  "title": "Link comms relay",
  "status": "in_progress",
  "priority": "elevated",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

GET {{base_url}}/v1/missions/{{mission_id}}/proto
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/x-protobuf

GET {{base_url}}/v1/tasks/{{task_id}}/proto
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/x-protobuf

DELETE {{base_url}}/v1/tasks/{{task_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204

DELETE {{base_url}}/v1/missions/{{mission_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204
