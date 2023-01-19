-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN customer_id VARCHAR(100) NOT NULL DEFAULT false;
