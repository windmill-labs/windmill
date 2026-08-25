-- Add up migration script here

ALTER TABLE gcp_trigger ALTER COLUMN gcp_resource_path DROP NOT NULL;
ALTER TABLE gcp_trigger ADD COLUMN project_id VARCHAR(255);

-- Subscriptions are now stored fully qualified (projects/<project>/subscriptions/<id>) so that one
-- Pub/Sub subscription has exactly one representation. A Pub/Sub id may itself be 255 characters,
-- so the qualified form needs more room than the bare one did.
ALTER TABLE gcp_trigger ALTER COLUMN subscription_id TYPE VARCHAR(400);
ALTER TABLE gcp_trigger DROP CONSTRAINT gcp_trigger_subscription_id_check;
ALTER TABLE gcp_trigger ADD CONSTRAINT gcp_trigger_subscription_id_check
    CHECK (char_length(subscription_id::text) >= 3 AND char_length(subscription_id::text) <= 400);

-- A NULL gcp_resource_path means application default credentials, and NULLs compare as distinct,
-- so the plain column index would stop guarding those rows. project_id is deliberately absent:
-- with the subscription stored fully qualified it is not part of a subscription's identity, and
-- including it would split rows that denote the same subscription.
--
-- Rows written before this migration hold a bare id, which cannot be backfilled here: the project
-- they resolve against lives inside the credentials, not in this table. So a legacy `my-sub` and a
-- new `projects/p/subscriptions/my-sub` still read as different subscriptions until the older
-- trigger is saved again, which rewrites it in the qualified form.
DROP INDEX unique_subscription_per_gcp_resource;
CREATE UNIQUE INDEX unique_subscription_per_gcp_resource
ON gcp_trigger (subscription_id, COALESCE(gcp_resource_path, ''), workspace_id);
