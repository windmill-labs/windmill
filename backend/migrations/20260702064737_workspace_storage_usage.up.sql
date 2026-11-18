-- Cached per-(workspace, storage) byte usage of workspace object storage,
-- refreshed by listing the storage location and adjusted optimistically as
-- uploads complete. Read on every workspace-storage write in CE builds to
-- enforce the storage quota, and by the storage_usage endpoint in all builds.
CREATE TABLE workspace_storage_usage (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace (id) ON DELETE CASCADE,
    storage VARCHAR(255) NOT NULL,
    bytes BIGINT NOT NULL DEFAULT 0,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, storage)
);

-- Tables created after the one-time GRANT ALL in 20250205131523 need explicit
-- grants: ALTER DEFAULT PRIVILEGES only covers objects created by the role
-- that set them (same gap as workspace_diff, notify_event, script_trigger).
GRANT ALL ON workspace_storage_usage TO windmill_user;
GRANT ALL ON workspace_storage_usage TO windmill_admin;
