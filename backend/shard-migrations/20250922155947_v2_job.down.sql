-- Add down migration script here

DROP TYPE IF EXISTS job_status CASCADE;
DROP TYPE IF EXISTS job_kind CASCADE;
DROP TYPE IF EXISTS job_trigger_kind CASCADE;
DROP TYPE IF EXISTS script_lang CASCADE;
DROP TABLE IF EXISTS v2_job;

