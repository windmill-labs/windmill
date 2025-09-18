-- Add up migration script here
-- Fix v2_job_completed constraints (from 20250201145629_v2_job_completed_constraints)

-- Set proper NOT NULL constraints and defaults
ALTER TABLE public.v2_job_completed ALTER COLUMN status SET NOT NULL;
ALTER TABLE public.v2_job_completed ALTER COLUMN completed_at SET DEFAULT now();
ALTER TABLE public.v2_job_completed ALTER COLUMN completed_at SET NOT NULL;
ALTER TABLE public.v2_job_completed ALTER COLUMN started_at DROP NOT NULL;