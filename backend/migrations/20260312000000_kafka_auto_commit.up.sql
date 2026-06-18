ALTER TABLE kafka_trigger ADD COLUMN auto_commit BOOLEAN NOT NULL DEFAULT TRUE;

CREATE TABLE kafka_pending_commits (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL,
    kafka_trigger_path VARCHAR(255) NOT NULL,
    topic VARCHAR(255) NOT NULL,
    partition INTEGER NOT NULL,
    "offset" BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (workspace_id, kafka_trigger_path) REFERENCES kafka_trigger(workspace_id, path) ON DELETE CASCADE
);

CREATE INDEX idx_kafka_pending_commits_trigger ON kafka_pending_commits (workspace_id, kafka_trigger_path);
