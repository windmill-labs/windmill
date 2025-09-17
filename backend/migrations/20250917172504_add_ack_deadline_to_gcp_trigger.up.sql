-- Add up migration script here
ALTER TABLE gcp_trigger ADD COLUMN ack_deadline INTEGER;