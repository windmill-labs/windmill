-- Add up migration script here
ALTER TABLE workspace_settings ADD COLUMN auto_invite_domain VARCHAR(50);
ALTER TABLE workspace DROP COLUMN domain;