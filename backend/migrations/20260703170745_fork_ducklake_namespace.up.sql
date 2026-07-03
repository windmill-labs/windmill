-- Registry of ducklake namespaces provisioned for fork/dev workspaces. One row per
-- (fork workspace, lake name): records the exact catalog metadata schema and data
-- sub-path the fork's jobs attach to, so fork deletion can drop the pg schema and
-- delete the S3 prefix deterministically (the row is written on first resolution,
-- before any physical state exists).
CREATE TABLE fork_ducklake_namespace (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    ducklake_name VARCHAR(255) NOT NULL,
    metadata_schema VARCHAR(63) NOT NULL,
    -- Named workspace storage holding the fork's data files; NULL = the default storage.
    storage TEXT,
    -- The fork namespace's data path within that storage (a bucket-root
    -- `__wm_forks/<fork wid>/…` prefix).
    data_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, ducklake_name)
);

-- Resolution runs under user_db transactions (SET LOCAL ROLE) in API contexts, so the
-- windmill roles need explicit grants (default privileges don't apply to app-created tables).
GRANT ALL ON fork_ducklake_namespace TO windmill_user;
GRANT ALL ON fork_ducklake_namespace TO windmill_admin;
