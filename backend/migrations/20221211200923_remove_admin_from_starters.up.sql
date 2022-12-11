-- Add up migration script here
DELETE FROM usr WHERE workspace_id = 'starter' AND email = 'admin@windmill.dev';