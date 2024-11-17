-- Add up migration script here
CREATE TABLE database_trigger(
    path VARCHAR(255) NOT NULL,
    script_path VARCHAR(255) NOT NULL,
    is_flow BOOLEAN NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    edited_by VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    edited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    extra_perms JSONB NOT NULL DEFAULT '{}',
    database JSONB NOT NULL,
    table_to_track JSONB[],
    error TEXT NULL,
    CONSTRAINT PK_database_trigger PRIMARY KEY (path,workspace_id)
);