-- Workspace storages a fork/dev workspace isolates from its parent: its writes land under
-- `__wm_forks/<fork segment>/` of that storage, and its reads fall back to the parent's keys.
-- One row per (fork, storage) chosen isolated at fork creation; forks created before this
-- table have none and keep sharing the parent's keys.
--
-- Deliberately NO foreign key to workspace(id), like fork_ducklake_namespace: rows are also the
-- cleanup ledger and must OUTLIVE the workspace row when deleting the prefix fails after the
-- delete commits. A recreated same-id fork would otherwise read the deleted fork's objects
-- under the deterministic prefix; fork creation retries leftover rows and refuses to proceed
-- while they cannot be cleaned.
CREATE TABLE fork_storage_namespace (
    workspace_id VARCHAR(50) NOT NULL,
    -- '' = the default storage, else a `secondary_storage` name.
    storage TEXT NOT NULL,
    -- The storage's resolved identity at fork creation (`<lfs type>:<resource path or root
    -- path>`), so cleanup deletes the prefix from the storage the fork wrote to even if the
    -- name is repointed later.
    storage_ref TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, storage)
);

GRANT ALL ON fork_storage_namespace TO windmill_user;
GRANT ALL ON fork_storage_namespace TO windmill_admin;
