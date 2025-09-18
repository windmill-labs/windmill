-- Add down migration script here
DROP TABLE IF EXISTS public.v2_job_completed;
DROP TYPE IF EXISTS v2_job_status;