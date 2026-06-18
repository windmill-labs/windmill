-- Add down migration script here
DROP FUNCTION v2_job_queue_before_insert() CASCADE;
DROP FUNCTION v2_job_queue_after_insert() CASCADE;
DROP FUNCTION v2_job_queue_before_update() CASCADE;
