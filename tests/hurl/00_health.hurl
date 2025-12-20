GET {{base_url}}/health
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.status" == "ok"

GET {{base_url}}/v1/status
HTTP 200
Content-Type: application/json
[Asserts]
jsonpath "$.service" exists
