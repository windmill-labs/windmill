-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'deploymentcallback';
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS git_sync JSONB;

CREATE TABLE deployment_metadata(
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    deployed_script_hash BIGINT NOT NULL,
    callback_job_ids UUID[],
    deployment_msg TEXT,
    PRIMARY KEY (workspace_id, deployed_script_hash)
);
