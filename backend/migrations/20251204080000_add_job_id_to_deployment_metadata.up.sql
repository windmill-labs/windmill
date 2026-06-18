-- Add job_id column to deployment_metadata table to track the current deployment job
ALTER TABLE deployment_metadata ADD COLUMN IF NOT EXISTS job_id UUID;
