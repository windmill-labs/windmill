-- Append-only event log for reactive asset-change triggers.
-- Written whenever a job produces/updates an asset ('w' or 'rw' access).
-- Stage 1 of asset-change implicit triggers plan.

CREATE TABLE asset_event (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE ON UPDATE CASCADE,
    asset_kind ASSET_KIND NOT NULL,
    asset_path VARCHAR(255) NOT NULL,
    partition_key VARCHAR(255),
    job_id UUID NOT NULL,
    script_hash BIGINT,
    access_type ASSET_ACCESS_TYPE NOT NULL,
    columns JSONB,
    metadata JSONB,
    at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX asset_event_kind_path_at_idx ON asset_event (workspace_id, asset_kind, asset_path, at DESC);
CREATE INDEX asset_event_job_id_idx ON asset_event (job_id);

GRANT ALL ON asset_event TO windmill_user;
GRANT ALL ON asset_event TO windmill_admin;
GRANT ALL ON asset_event_id_seq TO windmill_user;
GRANT ALL ON asset_event_id_seq TO windmill_admin;
