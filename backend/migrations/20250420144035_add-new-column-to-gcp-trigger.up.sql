-- Add up migration script here
CREATE TYPE GCP_SUBSCRIPTION_MODE AS ENUM ('create_update', 'existing');
ALTER TABLE gcp_trigger ADD COLUMN subscription_mode GCP_SUBSCRIPTION_MODE NOT NULL DEFAULT 'create_update'::GCP_SUBSCRIPTION_MODE;