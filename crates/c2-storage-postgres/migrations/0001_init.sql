CREATE TABLE IF NOT EXISTS missions (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    priority TEXT NOT NULL,
    classification TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_missions_tenant_created
    ON missions (tenant_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_missions_status
    ON missions (status);
CREATE INDEX IF NOT EXISTS idx_missions_classification
    ON missions (classification);

CREATE TABLE IF NOT EXISTS assets (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    classification TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_assets_tenant_created
    ON assets (tenant_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_assets_status
    ON assets (status);
CREATE INDEX IF NOT EXISTS idx_assets_kind
    ON assets (kind);

CREATE TABLE IF NOT EXISTS incidents (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    incident_type TEXT NOT NULL,
    status TEXT NOT NULL,
    summary TEXT NOT NULL,
    classification TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_incidents_tenant_created
    ON incidents (tenant_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_incidents_status
    ON incidents (status);
CREATE INDEX IF NOT EXISTS idx_incidents_type
    ON incidents (incident_type);

CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY,
    mission_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    priority TEXT NOT NULL,
    classification TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_mission_created
    ON tasks (mission_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_created
    ON tasks (tenant_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_status
    ON tasks (status);
