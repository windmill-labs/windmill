-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN customer_id VARCHAR(100);
ALTER TABLE workspace_settings ADD COLUMN plan VARCHAR(40);
