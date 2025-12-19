GET {{base_url}}/v1/mcp/capabilities
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json
jsonpath "$.version" exists

POST {{base_url}}/v1/mcp/handshake
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "client_id": "hurl-client",
  "desired": ["json", "protobuf"]
}
HTTP 200
Content-Type: application/json
jsonpath "$.session_id" exists

POST {{base_url}}/v1/mcp/sessions
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: application/json
{
  "client_id": "hurl-client",
  "mission_id": "{{mission_id}}"
}
HTTP 200
Content-Type: application/json
jsonpath "$.session_id" exists

POST {{base_url}}/v1/mcp/heartbeat
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
HTTP 200
Content-Type: application/json
jsonpath "$.status" == "ok"

POST {{base_url}}/v1/mcp/ingest
x-c2-tenant-id: {{tenant_id}}
x-c2-user-id: {{user_id}}
x-c2-roles: {{roles}}
x-c2-permissions: {{permissions}}
x-c2-clearance: {{clearance}}
Content-Type: text/plain
sensor=alpha;value=42
HTTP 202
Content-Type: application/json
jsonpath "$.status" == "accepted"
