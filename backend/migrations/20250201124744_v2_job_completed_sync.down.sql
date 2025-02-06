-- Add down migration script here
DROP FUNCTION v2_job_completed_before_insert() CASCADE;
DROP FUNCTION v2_job_completed_before_update() CASCADE;
