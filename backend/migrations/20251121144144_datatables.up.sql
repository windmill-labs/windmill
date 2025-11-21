-- Add up migration script here

ALTER TABLE workspace_settings
ADD COLUMN datatable JSONB;
