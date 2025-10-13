-- Add up migration script here

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