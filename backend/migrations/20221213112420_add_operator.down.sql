-- Add down migration script here
ALTER TABLE workspace_invite DROP COLUMN operator;
ALTER TABLE workspace_settings DROP COLUMN auto_invite_operator;
