-- Add up migration script here

-- Create enum for native trigger service names
CREATE TYPE native_trigger_service AS ENUM ('nextcloud');
CREATE TYPE runnable_kind AS ENUM ('script', 'flow');
ALTER TYPE TRIGGER_KIND ADD VALUE IF NOT EXISTS 'nextcloud';
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'nextcloud';

-- Create native_triggers table
CREATE TABLE native_triggers (
    id BIGSERIAL PRIMARY KEY,
    service_name native_trigger_service NOT NULL,
    external_id VARCHAR(255) NOT NULL,
    runnable_path VARCHAR(255) NOT NULL,
    runnable_kind RUNNABLE_KIND NOT NULL,
    event_type JSONB NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    summary TEXT,
    metadata JSONB NULL,
    edited_by VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_native_triggers_external UNIQUE (service_name, workspace_id, external_id),
    CONSTRAINT uq_native_triggers_internal UNIQUE (service_name, workspace_id, id),
    CONSTRAINT fk_native_triggers_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspace(id) ON DELETE CASCADE
);

CREATE INDEX idx_native_triggers_service_workspace_external
    ON native_triggers (service_name, workspace_id, external_id);

CREATE INDEX idx_native_triggers_workspace_and_id
    ON native_triggers (service_name, workspace_id, id);

GRANT ALL ON native_triggers TO windmill_user;
GRANT ALL ON native_triggers TO windmill_admin;

ALTER TABLE native_triggers ENABLE ROW LEVEL SECURITY;



CREATE TABLE workspace_integrations (
    workspace_id VARCHAR(50) NOT NULL,
    service_name native_trigger_service NOT NULL,
    oauth_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(50) NOT NULL,
    PRIMARY KEY (workspace_id, service_name),
    CONSTRAINT fk_workspace_integrations_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspace(id) ON DELETE CASCADE
);

CREATE INDEX idx_workspace_integrations_workspace
    ON workspace_integrations (workspace_id);

CREATE INDEX idx_workspace_integrations_service
    ON workspace_integrations (service_name);

GRANT ALL ON workspace_integrations TO windmill_user;
GRANT ALL ON workspace_integrations TO windmill_admin;

ALTER TABLE workspace_integrations ENABLE ROW LEVEL SECURITY;