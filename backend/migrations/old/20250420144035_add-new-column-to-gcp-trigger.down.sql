-- Add down migration script here
ALTER TABLE gcp_trigger DROP COLUMN subscription_mode;
DROP TYPE GCP_SUBSCRIPTION_MODE;