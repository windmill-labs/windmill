-- Add up migration script here
CREATE TABLE IF NOT EXISTS v2_job_runtime (
    id          UUID REFERENCES queue (id) ON DELETE CASCADE PRIMARY KEY NOT NULL,
    -- Metrics fields:
    ping        TIMESTAMP WITH TIME ZONE DEFAULT now(),
    memory_peak INTEGER
);

CREATE POLICY admin_policy ON v2_job_runtime
    AS PERMISSIVE
    FOR ALL
    TO windmill_admin;

GRANT ALL ON v2_job_runtime TO windmill_user, windmill_admin;
