-- Add down migration script here
ALTER TABLE job RENAME TO v2_job;
CREATE OR REPLACE VIEW job AS (
    SELECT id, raw_code, raw_lock, raw_flow, tag, workspace_id
    FROM v2_job
);
