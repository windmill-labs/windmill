-- Add down migration script here
ALTER TABLE v2_job_completed ALTER COLUMN status DROP NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN completed_at DROP DEFAULT;
ALTER TABLE v2_job_completed ALTER COLUMN completed_at DROP NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN started_at SET NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN __created_at SET NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN __created_by SET NOT NULL;
ALTER TABLE v2_job_completed ALTER COLUMN __success SET NOT NULL;
