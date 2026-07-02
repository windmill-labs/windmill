-- Reservation for in-flight (initiated but not yet completed) multipart uploads
-- to workspace object storage. Their parts occupy object-store capacity but are
-- invisible to the list-based storage recount until completion, so CE folds this
-- reservation into the remaining quota to bound abandoned uploads. Rows are
-- removed on complete/abort and lazily expired after a TTL.
--   inflight_bytes       - sum of the sizes of the parts uploaded so far
--   target_existing_size - size of the object the upload will overwrite (0 if new),
--                          credited so an overwrite only reserves the net growth
CREATE TABLE workspace_multipart_inflight (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace (id) ON DELETE CASCADE,
    upload_id VARCHAR(512) NOT NULL,
    storage VARCHAR(255) NOT NULL,
    inflight_bytes BIGINT NOT NULL DEFAULT 0,
    target_existing_size BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, upload_id)
);

CREATE INDEX idx_workspace_multipart_inflight_created_at
    ON workspace_multipart_inflight (created_at);

-- Tables created after the one-time GRANT ALL in 20250205131523 need explicit
-- grants: ALTER DEFAULT PRIVILEGES only covers objects created by the role that
-- set them (same gap as workspace_storage_usage, notify_event, script_trigger).
GRANT ALL ON workspace_multipart_inflight TO windmill_user;
GRANT ALL ON workspace_multipart_inflight TO windmill_admin;
