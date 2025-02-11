-- Add up migration script here
ALTER TABLE v2_job ALTER COLUMN workspace_id SET NOT NULL;
ALTER TABLE v2_job ALTER COLUMN tag SET DEFAULT 'other';
ALTER TABLE v2_job ALTER COLUMN tag SET NOT NULL;
