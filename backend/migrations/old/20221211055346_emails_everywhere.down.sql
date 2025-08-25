-- Add down migration script here
ALTER TABLE queue DROP COLUMN email; 
ALTER TABLE workspace_settings DROP COLUMN slack_email; 
ALTER TABLE schedule DROP COLUMN email; 
ALTER TABLE schedule DROP COLUMN error; 