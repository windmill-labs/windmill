-- Add up migration script here
CREATE TABLE database_trigger(
    path VARCHAR(255) NOT NULL,
    script_path VARCHAR(255) NOT NULL,
    is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    edited_by VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extra_perms JSONB NULL,
    database_resource_path VARCHAR(255) NOT NULL,
    error TEXT NULL,
    server_id VARCHAR(50) NULL,
    last_server_ping TIMESTAMPTZ NULL,
    replication_slot_name VARCHAR(255) NOT NULL,
    publication_name VARCHAR(255) NOT NULL,
    enabled BOOLEAN NOT NULL,
    CONSTRAINT PK_database_trigger PRIMARY KEY (path,workspace_id),
    CONSTRAINT fk_database_trigger_workspace FOREIGN KEY (workspace_id)
        REFERENCES workspace(id) ON DELETE CASCADE
);