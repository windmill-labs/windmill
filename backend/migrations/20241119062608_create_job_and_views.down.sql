-- Add down migration script here
DROP TABLE job CASCADE;
DROP VIEW queue_view CASCADE;
DROP VIEW completed_job_view CASCADE;
