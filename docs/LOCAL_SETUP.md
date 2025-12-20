# Local Setup

This guide covers local runtime setup for the C2 services and dependencies.

## SurrealDB (Primary Store)

```sh
surreal start --log info --user root --pass root --bind 0.0.0.0:8000 memory
```

Default env vars (override as needed):

- `C2_SURREAL_ENDPOINT=127.0.0.1:8000`
- `C2_SURREAL_NAMESPACE=c2`
- `C2_SURREAL_DATABASE=operations`
- `C2_SURREAL_USERNAME=root`
- `C2_SURREAL_PASSWORD=root`

## Postgres / Timescale (Adapters)

Ensure a database and user exist. Example:

```sh
createdb c2
createuser c2
```

Env vars:

- `C2_POSTGRES_URL=postgres://c2:changeme@localhost:5432/c2`
- `C2_TIMESCALE_URL=postgres://c2:changeme@localhost:5432/c2`

Migrations are applied automatically on connect.

## ZeroMQ (Messaging Bus)

Install libzmq and ensure it is in your build image.

Suggested env vars:

- `C2_ZMQ_PUB_ENDPOINT=tcp://127.0.0.1:5556`
- `C2_ZMQ_PUB_BIND=true`
- `C2_ZMQ_SUB_ENDPOINT=tcp://127.0.0.1:5556`
- `C2_ZMQ_SUB_BIND=false`
- `C2_ZMQ_SUB_TOPICS=c2.events`

## Run Services

```sh
cargo run -p c2-api
cargo run -p c2-mcp
```

## Web Console (UI)

The UI proxies API requests using headers configured via env vars:

```sh
export C2_UI_TENANT_ID=00000000-0000-0000-0000-000000000001
export C2_UI_USER_ID=00000000-0000-0000-0000-000000000002
export C2_UI_ROLES=system_admin
export C2_UI_PERMISSIONS=view_missions,edit_missions,dispatch_assets,view_incidents,ingest_data,access_classified,admin
export C2_UI_CLEARANCE=top_secret
```

Optional overrides:

- `C2_API_BASE_URL=http://c2-api:8080`
- `C2_UI_POLL_INTERVAL_MS=2000`
- `C2_UI_LIST_LIMIT=200`
- `C2_WEB_STATIC_DIR=services/c2-web/static`

## Metrics (Prometheus)

Set a metrics listener per service if you want to scrape metrics (use a port distinct from the HTTP service port):

```sh
export C2_METRICS_ADDR=0.0.0.0:9000
```

## API Tests (Hurl)

```sh
hurl --variables-file tests/hurl/c2.env tests/hurl/*.hurl
```
