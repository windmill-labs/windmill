-- Add up migration script here
ALTER TABLE v2_job_completed ALTER COLUMN status SET NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN completed_at SET DEFAULT now();
ALTER TABLE v2_job_completed ALTER COLUMN completed_at SET NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN started_at DROP NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN __created_at DROP NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN __created_by DROP NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN __success DROP NOT NULL;
