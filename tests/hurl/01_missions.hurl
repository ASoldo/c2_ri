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
  "name": "Operation Torch",
  "status": "planned",
  "priority": "routine",
  "classification": "unclassified",
  "created_at_ms": {{created_at_ms}},
  "updated_at_ms": {{updated_at_ms}}
}
HTTP 200

GET {{base_url}}/v1/missions/{{mission_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.id" == "{{mission_id}}"

GET {{base_url}}/v1/missions?tenant_id={{tenant_id}}&limit=10&offset=0
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json

DELETE {{base_url}}/v1/missions/{{mission_id}}
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 204
