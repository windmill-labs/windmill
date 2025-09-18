-- Add down migration script here
-- Remove V2 job completed sync triggers

DROP TRIGGER IF EXISTS v2_job_completed_before_insert_trigger ON v2_job_completed;
DROP TRIGGER IF EXISTS v2_job_completed_before_update_trigger ON v2_job_completed;

DROP FUNCTION IF EXISTS v2_job_completed_before_insert();
DROP FUNCTION IF EXISTS v2_job_completed_before_update();