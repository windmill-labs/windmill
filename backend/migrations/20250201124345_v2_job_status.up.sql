-- Add up migration script here
CREATE TABLE IF NOT EXISTS v2_job_status (
    id                      UUID REFERENCES queue (id) ON DELETE CASCADE PRIMARY KEY NOT NULL,
    -- Flow status fields:
    flow_status             JSONB,
    flow_leaf_jobs          JSONB,
    -- Workflow as code fields:
    workflow_as_code_status JSONB
);

CREATE POLICY admin_policy ON v2_job_status
    AS PERMISSIVE
    FOR ALL
    TO windmill_admin;

GRANT ALL ON v2_job_status TO windmill_user, windmill_admin;
