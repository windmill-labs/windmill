-- Add down migration script here

DROP INDEX unique_subscription_per_gcp_resource;

DELETE FROM gcp_trigger WHERE gcp_resource_path IS NULL;

ALTER TABLE gcp_trigger DROP COLUMN project_id;
ALTER TABLE gcp_trigger ALTER COLUMN gcp_resource_path SET NOT NULL;

CREATE UNIQUE INDEX unique_subscription_per_gcp_resource
ON gcp_trigger (subscription_id, gcp_resource_path, workspace_id);
