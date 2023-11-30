-- Add up migration script here
ALTER TYPE JOB_KIND ADD VALUE IF NOT EXISTS 'deploymentcallback';
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS git_sync JSONB;

CREATE TABLE deployment_callback_runs(
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    deployed_script_hash BIGINT NOT NULL,
    job_id UUID NOT NULL,
    callback_script_path VARCHAR(255) NOT NULL,
    deployment_msg TEXT,
    PRIMARY KEY (workspace_id, deployed_script_hash)
);
