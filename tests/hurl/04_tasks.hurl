POST {{base_url}}/v1/missions
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "id": "{{task_mission_id}}",
  "tenant_id": "{{tenant_id}}",
  "name": "Operation Ember",
  "status": "active",
  "priority": "urgent",
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
  "mission_id": "{{task_mission_id}}",
  "tenant_id": "{{tenant_id}}",
  "title": "Establish perimeter at grid 3C",
  "status": "pending",
  "priority": "urgent",
  "classification": "restricted",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

GET {{base_url}}/v1/tasks/{{task_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.id" == "{{task_id}}"

GET {{base_url}}/v1/missions/{{task_mission_id}}/tasks?limit=10&offset=0
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json

DELETE {{base_url}}/v1/tasks/{{task_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204

DELETE {{base_url}}/v1/missions/{{task_mission_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204
