-- Add up migration script here

-- Create enum for native trigger service names
CREATE TYPE native_trigger_service AS ENUM ('nextcloud');
CREATE TYPE runnable_kind AS ENUM ('script', 'flow');


-- Create native_triggers table
CREATE TABLE native_triggers (
    id BIGSERIAL PRIMARY KEY,
    service_name native_trigger_service NOT NULL,
    external_id VARCHAR(255) NOT NULL,
    runnable_path VARCHAR(255) NOT NULL,
    runnable_kind RUNNABLE_KIND NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    resource_path VARCHAR(255) NOT NULL,
    summary TEXT NOT NULL,
    metadata JSONB NULL,
    edited_by VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_native_triggers_external UNIQUE (service_name, workspace_id, external_id),
    CONSTRAINT uq_native_triggers_internal UNIQUE (service_name, workspace_id, id),
    CONSTRAINT fk_native_triggers_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspace(id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX idx_native_triggers_service_workspace_external
    ON native_triggers (service_name, workspace_id, external_id);

CREATE INDEX idx_native_triggers_workspace_and_id
    ON native_triggers (workspace_id, id);

-- Grant permissions
GRANT ALL ON native_triggers TO windmill_user;
GRANT ALL ON native_triggers TO windmill_admin;

-- Enable row level security
ALTER TABLE native_triggers ENABLE ROW LEVEL SECURITY;

-- Add nextcloud to job_trigger_kind enum
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'nextcloud';
