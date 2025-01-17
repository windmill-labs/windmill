-- Add up migration script here
CREATE TABLE IF NOT EXISTS v2_job_flow_runtime (
    id          UUID REFERENCES queue (id) ON DELETE CASCADE PRIMARY KEY NOT NULL,
    -- Flow status fields:
    flow_status JSONB                                                    NOT NULL,
    leaf_jobs   JSONB
);

CREATE POLICY admin_policy ON v2_job_flow_runtime
    AS PERMISSIVE
    FOR ALL
    TO windmill_admin;

GRANT ALL ON v2_job_flow_runtime TO windmill_user, windmill_admin;
