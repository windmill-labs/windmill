-- Remove job_id column from deployment_metadata table
ALTER TABLE deployment_metadata DROP COLUMN IF EXISTS job_id;
