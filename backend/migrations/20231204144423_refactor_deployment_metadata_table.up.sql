-- Add up migration script here
DROP TABLE deployment_metadata;

CREATE TABLE deployment_metadata(
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    path VARCHAR(255) NOT NULL,
    script_hash BIGINT,
    app_version BIGINT,
    callback_job_ids UUID[],
    deployment_msg TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS deployment_metadata_script ON deployment_metadata (workspace_id, script_hash) WHERE script_hash IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS deployment_metadata_flow ON deployment_metadata (workspace_id, path) WHERE script_hash IS NULL AND app_version IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS deployment_metadata_app ON deployment_metadata (workspace_id, path, app_version) WHERE app_version IS NOT NULL;
