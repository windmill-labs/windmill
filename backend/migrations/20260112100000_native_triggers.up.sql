-- Add up migration script here

CREATE TYPE native_trigger_service AS ENUM ('nextcloud');
ALTER TYPE TRIGGER_KIND ADD VALUE IF NOT EXISTS 'nextcloud';
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'nextcloud';

CREATE TABLE native_trigger (
    external_id VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    service_name native_trigger_service NOT NULL,
    script_path VARCHAR(255) NOT NULL,
    is_flow BOOLEAN NOT NULL,
    webhook_token_prefix VARCHAR(10) NOT NULL,
    service_config JSONB,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (external_id, workspace_id, service_name),
    CONSTRAINT fk_native_trigger_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspace(id) ON DELETE CASCADE
);

CREATE INDEX idx_native_trigger_workspace
    ON native_trigger (workspace_id);

CREATE INDEX idx_native_trigger_script_path
    ON native_trigger (workspace_id, script_path, is_flow);

GRANT ALL ON native_trigger TO windmill_user;
GRANT ALL ON native_trigger TO windmill_admin;


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
