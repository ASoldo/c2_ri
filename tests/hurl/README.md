# Hurl API Tests

These scripts validate the HTTP API surface (health, CRUD, protobuf, streams, MCP HTTP routes).

## Prerequisites

- Start SurrealDB (primary store):
  - `surreal start --log info --user root --pass root --bind 0.0.0.0:8000 memory`
- Start c2-api:
  - `cargo run -p c2-api`
- Optional: set `base_url` in `tests/hurl/c2.env` if the API is on a different host/port.

## Run

```sh
hurl --variables-file tests/hurl/c2.env tests/hurl/*.hurl
```

## Notes

- WebSocket and SSE tests validate headers and status only; payload streaming is tested separately.
- MCP protocol tests run against `c2-api` HTTP routes. The rmcp server runs in `c2-mcp`.
