CREATE TABLE IF NOT EXISTS units (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    display_name TEXT NOT NULL,
    callsign TEXT,
    readiness TEXT NOT NULL,
    comms_status TEXT NOT NULL,
    classification TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_units_tenant_created
    ON units (tenant_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_units_readiness
    ON units (readiness);

CREATE TABLE IF NOT EXISTS teams (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    callsign TEXT,
    classification TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_teams_tenant_created
    ON teams (tenant_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_teams_name
    ON teams (name);

CREATE TABLE IF NOT EXISTS capabilities (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    classification TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_capabilities_tenant_created
    ON capabilities (tenant_id, created_at_ms DESC);
CREATE INDEX IF NOT EXISTS idx_capabilities_code
    ON capabilities (code);
