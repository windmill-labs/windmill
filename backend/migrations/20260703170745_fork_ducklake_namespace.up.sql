-- Registry of ducklake namespaces provisioned for fork/dev workspaces. One row per
-- (fork workspace, lake name): records the exact catalog metadata schema and data
-- sub-path the fork's jobs attach to, so fork deletion can drop the pg schema and
-- delete the S3 prefix deterministically (the row is written on first resolution,
-- before any physical state exists).
CREATE TABLE fork_ducklake_namespace (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    ducklake_name VARCHAR(255) NOT NULL,
    metadata_schema VARCHAR(63) NOT NULL,
    -- Canonical identity of the catalog database the metadata schema lives in
    -- (`<resource_type>:<resource_path>`, e.g. `instance:wm_ducklake` or
    -- `postgres:u/admin/pg`). Cleanup connects to THIS catalog, not whatever the fork's
    -- settings point at by then — a drifted catalog resource must not make cleanup drop a
    -- schema in the wrong database and orphan the real one.
    catalog TEXT NOT NULL,
    -- Named workspace storage holding the fork's data files; '' = the default storage
    -- (part of the PK, which cannot hold NULL).
    storage TEXT NOT NULL DEFAULT '',
    -- The storage's RESOLVED identity at registration time (`<lfs type>:<resource path or
    -- root path>`, e.g. `s3:u/admin/minio` or `filesystem:/data/lfs`; '' = unknown). Cleanup
    -- deletes the fork prefix from THIS storage, not whatever the logical name points at by
    -- then — repointing a storage after attach must not orphan the original fork data (or
    -- delete a colliding prefix from the new one).
    storage_ref TEXT NOT NULL DEFAULT '',
    -- The fork namespace's data path within that storage (a bucket-root
    -- `__wm_forks/<fork wid>/…` prefix).
    data_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- One row per physical location EVER attached: if the fork's lake settings drift
    -- (catalog/storage/path change), later attaches add rows rather than replace them, so
    -- cleanup covers every location the fork wrote, not just the first.
    PRIMARY KEY (workspace_id, ducklake_name, catalog, storage, storage_ref, data_path)
);

-- Resolution runs under user_db transactions (SET LOCAL ROLE) in API contexts, so the
-- windmill roles need explicit grants (default privileges don't apply to app-created tables).
GRANT ALL ON fork_ducklake_namespace TO windmill_user;
GRANT ALL ON fork_ducklake_namespace TO windmill_admin;
