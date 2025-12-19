# Progress Log

## Week of 2025-12-19

Highlights
- Added policy enforcement, tasks CRUD, and SurrealDB task repository
- Added protobuf, websocket, SSE, and MCP integration endpoints
- Implemented Postgres adapter and Timescale wrapper for JSON payload storage
- Resolved storage adapter wiring and protobuf enum naming for clean builds
- Added rmcp-based MCP service with tools/resources and auth enforcement
- Added Postgres/Timescale migrations plus SurrealDB schema bootstrap
- Added ZeroMQ transport module and reproducible Hurl API test suite

Feature IDs Touched
- FND-006
- FND-007
- FND-004
- CMD-001
- CMD-002
- CMD-003
- CMD-004
- UI-001
- DATA-001
- DATA-002
- DATA-003
- INT-004
- INT-005
- MSG-005
- MSG-006
- MSG-001
- SEC-001
- OPS-005

Risks and Blockers
- Pingora auth is token-based; needs full authn/authz policy integration
- MCP and stream routes are placeholders pending real ingest pipelines
- Storage adapters need migrations, indexing, and schema hardening
- MCP auth currently relies on explicit tool/meta auth payloads until tokenized service identity
- ZeroMQ transport not yet wired into worker ingestion loops

Next Focus
- Add migrations and index strategy for Surreal/Postgres/Timescale
- Wire real-time streams to messaging backbone
- Define MCP contract and ingestion pipeline implementation
- Integrate ZeroMQ bus into worker ingest and publish paths

## Week of 2025-12-26

Highlights
- Added c2-operator service with CRD reconciliation for core services
- Added CRD generator plus Kubernetes operator and cluster manifests
- Added multi-service Docker build definition and deployment guide
- Wired Prometheus exporter and Grafana/Prometheus deployment manifests
- Added Kustomize overlays and ArgoCD application templates for GitOps deployment
- Aligned ArgoCD repo URL and consolidated observability manifests under k8s/
- Added dev overlay SurrealDB manifest for minikube testing
- Added Harbor registry setup and dev overlay registry wiring
- Added Keycloak helm values and LDAP setup guidance

Feature IDs Touched
- FND-008
- OPS-006
- OPS-007
- OPS-008
- OPS-009
- SEC-006

Risks and Blockers
- Operator defaults still need probe wiring and TLS secret management
- CRD database credentials rely on env/secret wiring until external secret integration

Next Focus
- Add readiness/liveness probes and resource defaults in operator
- Expand operator support for gateway TLS and secret mounting
