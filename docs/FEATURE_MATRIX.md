# Feature Matrix

This matrix captures baseline C2 capabilities for military and first responder operations.
IDs are stable and used to track design, implementation, and changelog entries.

## Foundation and Platform

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| FND-001 | Multi-tenant tenancy model and service identity | All | P0 | c2-api, c2-worker | c2-core, c2-identity | Planned |
| FND-002 | Config from env and K8s secrets | Ops | P0 | All | c2-config | In progress |
| FND-003 | Observability bootstrapping (logs, metrics hooks) | Ops | P0 | All | c2-observability | In progress |
| FND-006 | Actix service scaffolding for API endpoints | Ops | P0 | c2-api | c2-config, c2-observability | In progress |
| FND-007 | Pingora gateway ingress and routing baseline | Ops | P0 | c2-gateway | c2-config | In progress |
| FND-004 | Service-to-service authn/authz baseline | Ops | P0 | c2-api, c2-gateway | c2-identity, c2-policy | In progress |
| FND-005 | Audit-ready error model and codes | Ops | P1 | c2-api | c2-core | Planned |
| FND-008 | Container build and runtime images for services | Ops | P0 | All | docker | In progress |

## Identity and Security

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| SEC-001 | Role based access control and clearance tiers | Command, Ops | P0 | c2-api | c2-identity, c2-policy | In progress |
| SEC-002 | Attribute based policy evaluation | Command, Ops | P0 | c2-api, c2-worker | c2-policy | Planned |
| SEC-003 | Token claims, session controls, expiry enforcement | Ops | P0 | c2-api, c2-gateway | c2-identity | Planned |
| SEC-004 | Data classification enforcement for records | Command, Ops | P0 | c2-api, c2-worker | c2-core | Planned |
| SEC-005 | Immutable audit log pipeline | Compliance | P1 | c2-worker | c2-storage, c2-messaging | Planned |
| SEC-006 | SSO and LDAP federation (Keycloak) | Ops | P0 | c2-api, c2-gateway | c2-identity | In progress |
| SEC-007 | Resource-level authorization enforcement | Command, Ops | P0 | c2-api, c2-worker | c2-policy | Planned |
| SEC-008 | Tamper-evident audit log hashing | Compliance | P1 | c2-worker | c2-storage, c2-core | Planned |
| SEC-009 | Retention policies and legal hold controls | Compliance | P1 | c2-worker | c2-storage | Planned |

## Command and Mission Management

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| CMD-001 | Mission lifecycle management | Command | P0 | c2-api, c2-web | c2-core, c2-storage | In progress |
| CMD-002 | Tasking and assignment workflow | Command | P0 | c2-api, c2-web | c2-core, c2-storage | In progress |
| CMD-003 | Asset registry and readiness state | Ops | P0 | c2-api, c2-web | c2-core, c2-storage | In progress |
| CMD-004 | Incident intake and response tracking | Ops, Field | P0 | c2-api, c2-web | c2-core, c2-storage | In progress |
| CMD-005 | Command approvals and escalation workflows | Command | P1 | c2-api, c2-web | c2-policy | Planned |
| CMD-006 | Incident escalation runbooks and playbooks | Command, Ops | P1 | c2-api, c2-web | c2-core, c2-policy | Planned |

## Operational Data Model

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| ODM-001 | Unit registry with readiness state | Ops | P0 | c2-api, c2-web | c2-core, c2-storage | In progress |
| ODM-002 | Team and detachment registry | Command, Ops | P0 | c2-api, c2-web | c2-core, c2-storage | In progress |
| ODM-003 | Capability catalog and capability tagging | Command, Ops | P1 | c2-api, c2-web | c2-core, c2-storage | In progress |
| ODM-004 | Asset maintenance lifecycle and scheduling fields | Ops | P0 | c2-api, c2-worker | c2-core, c2-storage | In progress |
| ODM-005 | Comms status and link health | Ops, Field | P0 | c2-api, c2-worker | c2-core, c2-storage | In progress |
| ODM-006 | Operational readiness scoring and posture | Command, Ops | P1 | c2-api, c2-worker | c2-core, c2-storage | Planned |

## Situational Awareness and Geo

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| SA-001 | Live asset location and status updates | Command, Field | P0 | c2-api, c2-worker | c2-geo, c2-messaging | Planned |
| SA-002 | Geofencing and boundary alerts | Command | P0 | c2-worker | c2-geo, c2-policy | Planned |
| SA-003 | Multi-layer map overlays and AOI regions | Command, Analyst | P1 | c2-web | c2-geo | Planned |
| SA-004 | Sensor and telemetry feed normalization | Analyst | P0 | c2-worker | c2-messaging, c2-storage | Planned |
| SA-005 | Operational timeline playback | Command, Analyst | P1 | c2-web | c2-storage | Planned |
| SA-006 | Track history storage and time-series queries | Command, Analyst | P0 | c2-worker | c2-geo, c2-storage-timescale | Planned |
| SA-007 | Geospatial track ingest and replay streams | Command, Field | P0 | c2-worker | c2-geo, c2-messaging | Planned |

## Messaging and Data Movement

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| MSG-001 | Message envelope standard and routing metadata (ZeroMQ transport) | Ops | P0 | c2-worker, c2-api | c2-messaging, c2-core | In progress |
| MSG-002 | Reliable event delivery with retries | Ops | P0 | c2-worker | c2-messaging | Planned |
| MSG-003 | Correlation IDs for cross-service tracing | Ops | P0 | All | c2-core, c2-messaging | Planned |
| MSG-004 | Data ingest pipelines for external feeds | Analyst | P1 | c2-worker | c2-messaging, c2-storage | Planned |
| MSG-005 | WebSocket real-time streams | Command, Ops | P0 | c2-api | c2-messaging | In progress |
| MSG-006 | Server-sent event streams | Command, Ops | P1 | c2-api | c2-messaging | In progress |

## Storage and Data Platforms

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| DATA-001 | SurrealDB primary operational store adapter | Ops | P0 | c2-api, c2-worker | c2-storage-surreal | In progress |
| DATA-002 | Postgres adapter for compatibility | Ops | P1 | c2-worker | c2-storage-postgres | In progress |
| DATA-003 | Timescale adapter for time-series telemetry | Ops | P1 | c2-worker | c2-storage-timescale | In progress |

## Collaboration and Workflow

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| COL-001 | Shared operational notes and annotations | Command, Field | P1 | c2-api, c2-web | c2-storage | Planned |
| COL-002 | Shift handover summaries and continuity | Ops | P1 | c2-web | c2-storage | Planned |
| COL-003 | Notification and alert routing | Ops, Field | P0 | c2-worker | c2-messaging, c2-policy | Planned |

## Operator UI

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| UI-001 | Operator web console shell and Tera templating | Command, Ops | P0 | c2-web | c2-config | In progress |
| UI-002 | Mission dashboards and role-based panels | Command, Ops | P0 | c2-web | c2-config | Planned |
| UI-003 | Alerts, notifications, and escalation panels | Command, Ops | P0 | c2-web | c2-config | Planned |
| UI-004 | Playbooks, runbooks, and checklists | Ops | P1 | c2-web | c2-config | Planned |
| UI-005 | Role-based layouts and access scoping | Command, Ops | P1 | c2-web | c2-config | Planned |
| UI-006 | Reporting, exports, and templated briefs | Command | P1 | c2-web | c2-config | Planned |

## Operations and Continuity

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| OPS-001 | Offline tolerant workflow with store-and-forward | Field | P0 | c2-worker | c2-messaging, c2-storage | Planned |
| OPS-002 | Multi-region failover aware routing | Ops | P1 | c2-gateway | c2-config | Planned |
| OPS-003 | Backup, restore, and data retention controls | Ops | P1 | c2-worker | c2-storage | Planned |
| OPS-004 | Health checks and readiness probes | Ops | P0 | All | c2-observability | In progress |
| OPS-005 | API smoke and regression test harness | Ops | P1 | c2-api | c2-config | In progress |
| OPS-007 | Prometheus and Grafana observability stack | Ops | P0 | All | c2-observability | In progress |
| OPS-008 | GitOps deployment overlays with ArgoCD and Kustomize | Ops | P0 | All | k8s | In progress |
| OPS-009 | Harbor registry integration for on-prem image distribution | Ops | P0 | All | k8s | In progress |
| OPS-006 | Kubernetes operator and CRD deployment controller | Ops | P0 | c2-operator | c2-operator | In progress |
| OPS-010 | Edge sync and conflict resolution | Field | P0 | c2-worker | c2-storage, c2-messaging | Planned |
| OPS-011 | Multi-cluster HA and failover orchestration | Ops | P1 | c2-gateway | c2-config | Planned |
| OPS-012 | Disaster recovery runbooks and restore validation | Ops | P1 | c2-worker | c2-storage | Planned |

## Interoperability and Integration

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| INT-001 | External system adapters and connectors | Integrator | P1 | c2-worker | c2-messaging | Planned |
| INT-002 | Standards based message schemas | Integrator | P1 | c2-worker | c2-messaging | Planned |
| INT-003 | Export controls and data sharing policies | Command | P1 | c2-api | c2-policy | Planned |
| INT-004 | Protobuf API payloads | Integrator | P1 | c2-api | c2-proto | In progress |
| INT-005 | MCP integration routes | Integrator | P1 | c2-api, c2-mcp | c2-policy | In progress |
| INT-006 | Public safety standards adapters (ICS/NIMS) | Integrator | P1 | c2-worker | c2-messaging | Planned |
| INT-007 | Military standards adapters (STANAG/JC3IEDM) | Integrator | P1 | c2-worker | c2-messaging | Planned |
| INT-008 | Radio/dispatch and CAD integration | Integrator | P1 | c2-worker | c2-messaging | Planned |

## Analytics and Reporting

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| ANA-001 | Mission outcome reporting | Command | P1 | c2-api, c2-web | c2-storage | Planned |
| ANA-002 | Asset utilization metrics | Ops | P1 | c2-api, c2-web | c2-storage | Planned |
| ANA-003 | Incident response time analytics | Command | P1 | c2-api, c2-web | c2-storage | Planned |
