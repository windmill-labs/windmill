-- Add down migration script here
-- Revert v2_job_completed constraints
ALTER TABLE public.v2_job_completed ALTER COLUMN status DROP NOT NULL;
ALTER TABLE public.v2_job_completed ALTER COLUMN completed_at DROP DEFAULT;
ALTER TABLE public.v2_job_completed ALTER COLUMN completed_at DROP NOT NULL;
ALTER TABLE public.v2_job_completed ALTER COLUMN started_at SET NOT NULL;