-- Add up migration script here
DROP FUNCTION v2_completed_job_instead_of_update_overlay() CASCADE;
DROP FUNCTION v2_completed_job_instead_of_update() CASCADE;
DROP FUNCTION v2_completed_job_instead_of_delete() CASCADE;
DROP FUNCTION v2_completed_job_update(OLD v2_completed_job, NEW v2_completed_job) CASCADE;
DROP VIEW v2_completed_job;
