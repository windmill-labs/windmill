CREATE TYPE sandbox_status AS ENUM ('creating', 'running', 'suspended', 'stopped', 'error');

CREATE TABLE sandbox_host (
    id VARCHAR(255) PRIMARY KEY,
    base_url TEXT NOT NULL,
    last_ping TIMESTAMPTZ NOT NULL DEFAULT now(),
    capacity INT NOT NULL DEFAULT 10,
    active_count INT NOT NULL DEFAULT 0,
    labels JSONB NOT NULL DEFAULT '{}'
);

CREATE TABLE sandbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    created_by VARCHAR(55) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    image TEXT,
    timeout_secs INT,
    idle_timeout_secs INT,
    cpu_limit INT NOT NULL DEFAULT 1,
    memory_limit_mb INT NOT NULL DEFAULT 512,
    disk_limit_mb INT NOT NULL DEFAULT 1024,
    env_vars JSONB NOT NULL DEFAULT '{}',
    labels JSONB NOT NULL DEFAULT '{}',
    mounts JSONB NOT NULL DEFAULT '[]',
    network_enabled BOOLEAN NOT NULL DEFAULT false,

    mode VARCHAR(20) NOT NULL DEFAULT 'embedded',
    parent_job_id UUID,
    host_id VARCHAR(255) REFERENCES sandbox_host(id),

    status sandbox_status NOT NULL DEFAULT 'creating',
    pid INT,
    started_at TIMESTAMPTZ,
    last_activity_at TIMESTAMPTZ,
    suspended_at TIMESTAMPTZ,
    stopped_at TIMESTAMPTZ,
    error_message TEXT,

    ephemeral BOOLEAN NOT NULL DEFAULT false,
    auto_stop_after_secs INT,
    expires_at TIMESTAMPTZ
);

CREATE INDEX idx_sandbox_workspace_status ON sandbox(workspace_id, status);
CREATE INDEX idx_sandbox_host ON sandbox(host_id) WHERE status IN ('running', 'suspended');
CREATE INDEX idx_sandbox_expires ON sandbox(expires_at) WHERE expires_at IS NOT NULL;

CREATE TABLE sandbox_exec (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sandbox_id UUID NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    workspace_id VARCHAR(50) NOT NULL,

    command TEXT NOT NULL,
    cwd TEXT,
    env_vars JSONB,

    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    exit_code INT,
    stdout TEXT,
    stderr TEXT,
    duration_ms BIGINT,

    created_by VARCHAR(55) NOT NULL
);

CREATE INDEX idx_sandbox_exec_sandbox ON sandbox_exec(sandbox_id, started_at DESC);
