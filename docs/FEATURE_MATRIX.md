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
| FND-004 | Service-to-service authn/authz baseline | Ops | P0 | c2-api, c2-gateway | c2-identity, c2-policy | Planned |
| FND-005 | Audit-ready error model and codes | Ops | P1 | c2-api | c2-core | Planned |

## Identity and Security

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| SEC-001 | Role based access control and clearance tiers | Command, Ops | P0 | c2-api | c2-identity, c2-policy | In progress |
| SEC-002 | Attribute based policy evaluation | Command, Ops | P0 | c2-api, c2-worker | c2-policy | Planned |
| SEC-003 | Token claims, session controls, expiry enforcement | Ops | P0 | c2-api, c2-gateway | c2-identity | Planned |
| SEC-004 | Data classification enforcement for records | Command, Ops | P0 | c2-api, c2-worker | c2-core | Planned |
| SEC-005 | Immutable audit log pipeline | Compliance | P1 | c2-worker | c2-storage, c2-messaging | Planned |

## Command and Mission Management

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| CMD-001 | Mission lifecycle management | Command | P0 | c2-api, c2-web | c2-core, c2-storage | Planned |
| CMD-002 | Tasking and assignment workflow | Command | P0 | c2-api, c2-web | c2-core, c2-storage | Planned |
| CMD-003 | Asset registry and readiness state | Ops | P0 | c2-api, c2-web | c2-core, c2-storage | Planned |
| CMD-004 | Incident intake and response tracking | Ops, Field | P0 | c2-api, c2-web | c2-core, c2-storage | Planned |
| CMD-005 | Command approvals and escalation workflows | Command | P1 | c2-api, c2-web | c2-policy | Planned |

## Situational Awareness and Geo

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| SA-001 | Live asset location and status updates | Command, Field | P0 | c2-api, c2-worker | c2-geo, c2-messaging | Planned |
| SA-002 | Geofencing and boundary alerts | Command | P0 | c2-worker | c2-geo, c2-policy | Planned |
| SA-003 | Multi-layer map overlays and AOI regions | Command, Analyst | P1 | c2-web | c2-geo | Planned |
| SA-004 | Sensor and telemetry feed normalization | Analyst | P0 | c2-worker | c2-messaging, c2-storage | Planned |
| SA-005 | Operational timeline playback | Command, Analyst | P1 | c2-web | c2-storage | Planned |

## Messaging and Data Movement

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| MSG-001 | Message envelope standard and routing metadata | Ops | P0 | c2-worker, c2-api | c2-messaging, c2-core | In progress |
| MSG-002 | Reliable event delivery with retries | Ops | P0 | c2-worker | c2-messaging | Planned |
| MSG-003 | Correlation IDs for cross-service tracing | Ops | P0 | All | c2-core, c2-messaging | Planned |
| MSG-004 | Data ingest pipelines for external feeds | Analyst | P1 | c2-worker | c2-messaging, c2-storage | Planned |

## Storage and Data Platforms

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| DATA-001 | SurrealDB primary operational store adapter | Ops | P0 | c2-api, c2-worker | c2-storage-surreal | In progress |
| DATA-002 | Postgres adapter for compatibility | Ops | P1 | c2-worker | c2-storage-postgres | Planned |
| DATA-003 | Timescale adapter for time-series telemetry | Ops | P1 | c2-worker | c2-storage-timescale | Planned |

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

## Operations and Continuity

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| OPS-001 | Offline tolerant workflow with store-and-forward | Field | P0 | c2-worker | c2-messaging, c2-storage | Planned |
| OPS-002 | Multi-region failover aware routing | Ops | P1 | c2-gateway | c2-config | Planned |
| OPS-003 | Backup, restore, and data retention controls | Ops | P1 | c2-worker | c2-storage | Planned |
| OPS-004 | Health checks and readiness probes | Ops | P0 | All | c2-observability | Planned |

## Interoperability and Integration

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| INT-001 | External system adapters and connectors | Integrator | P1 | c2-worker | c2-messaging | Planned |
| INT-002 | Standards based message schemas | Integrator | P1 | c2-worker | c2-messaging | Planned |
| INT-003 | Export controls and data sharing policies | Command | P1 | c2-api | c2-policy | Planned |

## Analytics and Reporting

| ID | Capability | Primary Users | Priority | Services | Crates | Status |
| --- | --- | --- | --- | --- | --- | --- |
| ANA-001 | Mission outcome reporting | Command | P1 | c2-api, c2-web | c2-storage | Planned |
| ANA-002 | Asset utilization metrics | Ops | P1 | c2-api, c2-web | c2-storage | Planned |
| ANA-003 | Incident response time analytics | Command | P1 | c2-api, c2-web | c2-storage | Planned |
