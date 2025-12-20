# Project Plan

## Purpose

Build a production-grade, Kubernetes-native C2 system for military and first responder operations.
This plan defines how we track progress, changelogs, and feature matrix parity.

## Workstreams

- Foundation and platform (config, observability, service bootstrapping)
- Identity and policy (authn, authz, clearance, audit)
- Mission and tasking (missions, tasks, assets, incidents)
- Operational data model (units, teams, capabilities, readiness, comms)
- Messaging and data movement (event schema, routing, retries)
- Geo and situational awareness (location, geofencing, overlays)
- Storage and durability (repositories, retention, backup)
- UI and operator workflows (web console and command UX)
- Gateway and external access (ingress, TLS, rate limits)
- Resilience and offline operations (edge sync, failover, DR)

## Milestones

- M0: Workspace scaffolding, baseline crates, config and observability bootstrapping
- M1: Core domain models and storage interfaces, mission and incident APIs
- M1.5: Operational data model expansion (units, teams, capabilities, readiness/comms/maintenance)
- M2: Messaging backbone, event envelopes, ingest pipelines
- M3: Geo services, geofencing, and map overlays
- M4: Policy enforcement and audit trail
- M5: Operator UI integration and collaboration workflows
- M6: Interop connectors and standards based schemas

## Progress Tracking

- Update `docs/PROGRESS_LOG.md` weekly with highlights, risks, and feature IDs touched.
- Each change references Feature Matrix IDs (example: FND-002, SEC-001).
- Track status in `docs/FEATURE_MATRIX.md` (Planned, In progress, Done).

## Changelog Tracking

- Maintain `CHANGELOG.md` with an Unreleased section.
- Every merged change adds a bullet under Unreleased with Feature Matrix IDs.
- Group entries by category: Added, Changed, Fixed, Security, Operations.

## Plan Updates

- Review this plan at each milestone boundary.
- Record plan updates in `docs/PROGRESS_LOG.md` with the new milestone focus.
- Keep backlog items in `docs/FEATURE_MATRIX.md` with status Planned.

## Matrix Parity Rules

- Every new feature or capability must be added to `docs/FEATURE_MATRIX.md`.
- Every Feature Matrix ID must map to at least one crate or service owner.
- Every implementation change must reference the Feature Matrix ID in the changelog.
- Periodic parity check: compare code modules to matrix entries once per sprint.

## Next Execution Slice (Workflow + Geo)

- CMD-002/CMD-005/CMD-006: task assignment, approvals, and incident runbooks (API + storage).
- SA-001/SA-006/SA-007: track ingest pipeline and time-series storage with SSE fan-out.
- SEC-007: resource-level authorization enforcement in handlers.
- UI-002/UI-003: dashboards and alert panels tied to readiness and incident state.
