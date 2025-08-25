-- Add up migration script here

CREATE TYPE WORKSPACE_KEY_KIND AS ENUM ('cloud');

CREATE TABLE workspace_key (
    workspace_id  VARCHAR(50) NOT NULL REFERENCES workspace(id),
    kind WORKSPACE_KEY_KIND NOT NULL,
    key VARCHAR(255) NOT NULL DEFAULT 'changeme',
    PRIMARY KEY (workspace_id, kind)
);

INSERT INTO workspace_key SELECT id as workspace_id, 'cloud' as kind, 'changeme' as key FROM workspace;

