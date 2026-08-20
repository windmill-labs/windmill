-- Add down migration script here

DROP INDEX unique_subscription_per_gcp_resource;

DELETE FROM gcp_trigger WHERE gcp_resource_path IS NULL;

-- Rows that differed only by project_id, and rows whose subscription no longer fits the narrower
-- column, would both make the restored index or constraint fail after the column changes below.
DELETE FROM gcp_trigger WHERE char_length(subscription_id::text) > 255;
DELETE FROM gcp_trigger t USING gcp_trigger keep
WHERE t.ctid > keep.ctid
  AND t.subscription_id = keep.subscription_id
  AND t.gcp_resource_path = keep.gcp_resource_path
  AND t.workspace_id = keep.workspace_id;

ALTER TABLE gcp_trigger DROP COLUMN project_id;
ALTER TABLE gcp_trigger ALTER COLUMN gcp_resource_path SET NOT NULL;

ALTER TABLE gcp_trigger DROP CONSTRAINT gcp_trigger_subscription_id_check;
ALTER TABLE gcp_trigger ALTER COLUMN subscription_id TYPE VARCHAR(255);
ALTER TABLE gcp_trigger ADD CONSTRAINT gcp_trigger_subscription_id_check
    CHECK (char_length(subscription_id::text) >= 3 AND char_length(subscription_id::text) <= 255);

CREATE UNIQUE INDEX unique_subscription_per_gcp_resource
ON gcp_trigger (subscription_id, gcp_resource_path, workspace_id);
