-- Add down migration script here
ALTER TABLE gcp_trigger DROP COLUMN ack_deadline;