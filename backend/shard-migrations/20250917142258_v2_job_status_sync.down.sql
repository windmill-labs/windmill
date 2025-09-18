-- Add down migration script here
-- Remove V2 job status sync triggers

DROP TRIGGER IF EXISTS v2_job_status_before_insert_trigger ON v2_job_status;
DROP TRIGGER IF EXISTS v2_job_status_before_update_trigger ON v2_job_status;

DROP FUNCTION IF EXISTS v2_job_status_before_insert();
DROP FUNCTION IF EXISTS v2_job_status_before_update();