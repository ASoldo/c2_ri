# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

Added
- FND-006 Actix API service scaffolding with health and status endpoints
- FND-007 Pingora gateway routing baseline for API and web services
- UI-001 Web operator console shell with Tera templates
- DATA-001 SurrealDB primary adapter scaffold with env config
- DATA-002 Postgres adapter scaffold for compatibility
- DATA-003 Timescale adapter scaffold for time-series telemetry
- FND-004 Gateway auth token enforcement and optional TLS listener
- CMD-001 Mission CRUD endpoints backed by storage
- CMD-003 Asset CRUD endpoints backed by storage
- CMD-004 Incident CRUD endpoints backed by storage
- DATA-001 SurrealDB repository implementations for missions, assets, and incidents
- CMD-002 Task CRUD endpoints backed by storage
- INT-004 Protobuf payload endpoints for missions and tasks
- MSG-005 WebSocket stream endpoint
- MSG-006 Server-sent event stream endpoint
- INT-005 MCP integration routes
- SEC-001 Policy enforcement via BasicPolicyEngine
- DATA-002 Postgres adapter implementation with JSON payload storage
- DATA-003 Timescale adapter implementation using Postgres compatibility
- INT-005 rmcp-based MCP service for tools/resources
- DATA-002 Postgres migrations with structured columns
- DATA-003 Timescale hypertable bootstrap
- MSG-001 ZeroMQ transport module for messaging
- OPS-005 Hurl regression scripts for API endpoints
- FND-008 Multi-service Docker build with runtime dependencies
- OPS-006 Kubernetes operator with CRD, controller, and deployment manifests
- OPS-007 Prometheus metrics exporter and Grafana/Prometheus manifests
- OPS-008 Kustomize overlays and ArgoCD application templates
- OPS-009 Harbor registry setup and dev overlay registry wiring
- SEC-006 Keycloak deployment values for LDAP-backed auth
- ODM-001 Unit registry models, storage adapters, and API endpoints
- ODM-002 Team registry models, storage adapters, and API endpoints
- ODM-003 Capability catalog models, storage adapters, and API endpoints
- ODM-004 Asset maintenance and readiness/comms fields across stores
- SEC-007 Policy rules for unit/team/capability permissions
- OPS-005 Hurl coverage for units, teams, and capabilities
- UI-002 Layered infinite-board UI shell with 2D/3D canvases and dock panels
- UI-003 Realtime-ready ECS rendering loop with pin overlays and HUD stats
- UI-001 Local vendor assets for Tailwind v4, htmx, and three.js
- UI-002 Entity feeds wired into ECS with UI entity stream endpoint
- UI-002 OSM globe texture layer for 3D map sphere rendering
- UI-003 Modular c2-web UI runtime under services/c2-web/static/ui with main bootstrap and focused UI modules
- UI-003 Flight/satellite/ship icon overlays restored (plane mesh + sprite icons) alongside ECS particle markers
- SA-001 ADSB.lol flight provider support with ADS-B feed parsing for higher-density live aircraft previews

Changed
- Moved observability manifests to k8s/ overlays and updated ArgoCD repo refs
- Added dev overlay SurrealDB manifest for minikube testing
- UI-003 Overlay markers and edges now render via Three.js sprite overlay (DOM pin layers removed)
- UI-003 Three.js renderer now consumes WASM ECS render cache for marker positioning
- SA-001 Default flight feed now uses ADSB.lol (template URL) when no provider is configured

Fixed
- Disambiguated storage adapter trait calls across API and Timescale wrappers
- Cleaned up SSE stream typing and WebSocket actor wiring

Security
- 

Operations
- 
