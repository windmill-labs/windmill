-- Add up migration script here

ALTER TABLE gcp_trigger ALTER COLUMN gcp_resource_path DROP NOT NULL;
ALTER TABLE gcp_trigger ADD COLUMN project_id VARCHAR(255);

-- A NULL gcp_resource_path means application default credentials, and NULLs compare as distinct,
-- so the plain column index would stop guarding those rows. COALESCE keeps one trigger per
-- (subscription, credentials, project) the way it did when the path was mandatory.
DROP INDEX unique_subscription_per_gcp_resource;
CREATE UNIQUE INDEX unique_subscription_per_gcp_resource
ON gcp_trigger (subscription_id, COALESCE(gcp_resource_path, ''), COALESCE(project_id, ''), workspace_id);
