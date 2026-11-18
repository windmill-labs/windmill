-- Reservation for the parts of in-flight (initiated but not yet completed)
-- multipart uploads to workspace object storage. Uncommitted parts occupy
-- object-store capacity but are invisible to the list-based storage recount
-- until completion, so CE folds this reservation into the remaining quota to
-- bound abandoned uploads. One row per uploaded part so a re-uploaded part
-- (same part_id) replaces rather than double-counts; a part is recorded only
-- after its upstream upload succeeds. Rows are removed on successful complete
-- and lazily expired after a TTL (abort/abandon rely on the TTL, which matches
-- when the object store reaps the uncommitted parts).
--   part_id              - S3 part number or Azure block id (string)
--   part_bytes           - size of that part
--   target_existing_size - size of the object the upload will overwrite (0 if new),
--                          credited so an overwrite only reserves the net growth
CREATE TABLE workspace_multipart_inflight (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace (id) ON DELETE CASCADE,
    upload_id VARCHAR(512) NOT NULL,
    part_id VARCHAR(256) NOT NULL,
    storage VARCHAR(255) NOT NULL,
    part_bytes BIGINT NOT NULL DEFAULT 0,
    target_existing_size BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, upload_id, part_id)
);

CREATE INDEX idx_workspace_multipart_inflight_created_at
    ON workspace_multipart_inflight (created_at);

-- Tables created after the one-time GRANT ALL in 20250205131523 need explicit
-- grants: ALTER DEFAULT PRIVILEGES only covers objects created by the role that
-- set them (same gap as workspace_storage_usage, notify_event, script_trigger).
GRANT ALL ON workspace_multipart_inflight TO windmill_user;
GRANT ALL ON workspace_multipart_inflight TO windmill_admin;
