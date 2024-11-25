-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'flowscript';

-- Same as `flow_version` but with a "lite" value (raw scripts replaced by `flow_script`).
CREATE TABLE flow_version_lite (
    id BIGSERIAL PRIMARY KEY,
    value JSONB,
    FOREIGN KEY (id) REFERENCES flow_version (id) ON DELETE CASCADE
);

-- Either a script or a flow.
CREATE TABLE flow_node (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    hash BIGINT NOT NULL,
    path VARCHAR(255) NOT NULL, -- flow path.
    lock TEXT,
    code TEXT,
    flow JSONB,
    FOREIGN KEY (path, workspace_id) REFERENCES flow (path, workspace_id) ON DELETE CASCADE
);

CREATE INDEX flow_node_hash ON flow_node (hash);
